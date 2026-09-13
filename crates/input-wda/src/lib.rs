//! Loopback-only WebDriverAgent client. No Appium, Python, or Node runtime.
#[cfg(windows)]
pub mod runtime;
use imirror_coordinate_map::{Point, Size};
use imirror_input_core::{Button, Controller, Input};
use reqwest::{Method, Url, blocking::Client};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::Read,
    net::IpAddr,
    time::{Duration, Instant},
};

const MAX_REPLY: u64 = 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum WdaError {
    #[error(
        "WDA must use an HTTP address on 127.0.0.1 or ::1, without credentials, query or fragment"
    )]
    InvalidEndpoint,
    #[error("WDA connection failed; start the signed runner and USB forwarder")]
    Connection(#[source] reqwest::Error),
    #[error("WDA response could not be read")]
    Read(#[from] std::io::Error),
    #[error("WDA returned a malformed or oversized response")]
    InvalidResponse,
    #[error("WDA session expired; reconnect the signed runner")]
    SessionExpired,
    #[error("WDA rejected the command (HTTP {0})")]
    Rejected(u16),
    #[error("Input is outside the device, too large, or unsupported by WDA")]
    InvalidInput,
    #[error("WDA screen geometry changed with the session; reconnect control before tapping")]
    GeometryChanged,
}
pub struct Wda {
    client: Client,
    endpoint: Url,
    session: Option<String>,
    geometry_cache: Option<Size>,
    requests: u64,
    geometry_requests: u64,
    tap_requests: u64,
    tap_failures: u64,
    tap_durations: VecDeque<Duration>,
    last_geometry_request: Option<Duration>,
    pub last_request: Option<Duration>,
}
fn endpoint(text: &str) -> Result<Url, WdaError> {
    let url = Url::parse(text).map_err(|_| WdaError::InvalidEndpoint)?;
    let host = url
        .host_str()
        .ok_or(WdaError::InvalidEndpoint)?
        .trim_matches(['[', ']']);
    let ip: IpAddr = host.parse().map_err(|_| WdaError::InvalidEndpoint)?;
    if !ip.is_loopback()
        || url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(WdaError::InvalidEndpoint);
    }
    Ok(url)
}
impl Wda {
    pub fn new(address: &str) -> Result<Self, WdaError> {
        Self::with_timeout(address, Duration::from_secs(4))
    }
    pub fn with_timeout(address: &str, timeout: Duration) -> Result<Self, WdaError> {
        let endpoint = endpoint(address)?;
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2).min(timeout))
            .timeout(timeout)
            .build()
            .map_err(WdaError::Connection)?;
        Ok(Self {
            client,
            endpoint,
            session: None,
            geometry_cache: None,
            requests: 0,
            geometry_requests: 0,
            tap_requests: 0,
            tap_failures: 0,
            tap_durations: VecDeque::with_capacity(64),
            last_geometry_request: None,
            last_request: None,
        })
    }
    fn request(
        &mut self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, WdaError> {
        let start = Instant::now();
        self.requests += 1;
        self.geometry_requests += u64::from(path.ends_with("/window/size"));
        self.tap_requests += u64::from(path.ends_with("/wda/tap"));
        let result = self.request_inner(method, path, body);
        let elapsed = start.elapsed();
        self.last_request = Some(elapsed);
        if path.ends_with("/window/size") {
            self.last_geometry_request = Some(elapsed);
        }
        if result.is_err() {
            self.geometry_cache = None;
        }
        result
    }
    fn request_inner(
        &mut self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, WdaError> {
        let url = self
            .endpoint
            .join(path)
            .map_err(|_| WdaError::InvalidEndpoint)?;
        let mut request = self.client.request(method, url);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().map_err(WdaError::Connection);
        let response = response?;
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        response.take(MAX_REPLY + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_REPLY {
            return Err(WdaError::InvalidResponse);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| WdaError::InvalidResponse)?;
        if value.pointer("/value/error").and_then(Value::as_str) == Some("invalid session id")
            || value.get("status").and_then(Value::as_u64) == Some(6)
        {
            self.session = None;
            return Err(WdaError::SessionExpired);
        }
        if !(200..300).contains(&status)
            || value.pointer("/value/error").is_some()
            || value
                .get("status")
                .and_then(Value::as_u64)
                .is_some_and(|v| v != 0)
        {
            return Err(WdaError::Rejected(status));
        }
        Ok(value)
    }
    fn ensure_session(&mut self) -> Result<String, WdaError> {
        if let Some(session) = &self.session {
            return Ok(session.clone());
        }
        self.geometry_cache = None;
        let response=self.request(Method::POST,"session",Some(&json!({
            "capabilities":{"alwaysMatch":{"shouldWaitForQuiescence":false,"waitForIdleTimeout":0}}
        })))?;
        let session = response
            .get("sessionId")
            .or_else(|| response.pointer("/value/sessionId"))
            .and_then(Value::as_str)
            .ok_or(WdaError::InvalidResponse)?;
        if session.is_empty()
            || session.len() > 128
            || !session
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(WdaError::InvalidResponse);
        }
        self.session = Some(session.to_owned());
        Ok(session.to_owned())
    }
    fn session_request(
        &mut self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, WdaError> {
        let session = self.ensure_session()?;
        let previous_geometry = self.geometry_cache;
        let result = self.request(method.clone(), &format!("session/{session}/{path}"), body);
        // Only a definite invalid-session rejection is safe to replay. Never
        // repeat a tap/text/swipe after a timeout with an ambiguous outcome.
        if matches!(result, Err(WdaError::SessionExpired)) {
            let session = self.ensure_session()?;
            if method == Method::POST
                && matches!(path, "wda/tap" | "actions")
                && let Some(previous) = previous_geometry
                && self.geometry()? != previous
            {
                // The point was mapped against the old geometry. Don't replay
                // it onto a rotated/replaced session's coordinate system.
                return Err(WdaError::GeometryChanged);
            }
            self.request(method, &format!("session/{session}/{path}"), body)
        } else {
            result
        }
    }
    pub fn status(&mut self) -> Result<Value, WdaError> {
        self.request(Method::GET, "status", None)
    }
    fn cached_geometry(&mut self) -> Result<Size, WdaError> {
        match self.geometry_cache {
            Some(size) => Ok(size),
            None => self.geometry(),
        }
    }
    pub fn diagnostics(&self) -> Value {
        let mut samples: Vec<_> = self
            .tap_durations
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();
        samples.sort_by(f64::total_cmp);
        let percentile =
            |p: usize| (!samples.is_empty()).then(|| samples[(samples.len() - 1) * p / 100]);
        json!({"requests":self.requests,"geometry_requests":self.geometry_requests,
            "tap_requests":self.tap_requests,"tap_failures":self.tap_failures,
            "geometry_cached":self.geometry_cache.is_some(),
            "last_http_ms":self.last_request.map(|d|d.as_secs_f64()*1000.0),
            "last_geometry_request_ms":self.last_geometry_request.map(|d|d.as_secs_f64()*1000.0),
            "tap_dispatch":{"samples":samples.len(),"window_limit":64,
                "avg_ms":(!samples.is_empty()).then(||samples.iter().sum::<f64>()/samples.len() as f64),
                "p50_ms":percentile(50),"p95_ms":percentile(95),"p99_ms":percentile(99),
                "last_ms":self.tap_durations.back().map(|d|d.as_secs_f64()*1000.0)},
            "note":"Software WDA dispatch and HTTP timings, including failures; excludes UI queue time and physical display latency. No input contents are recorded."})
    }
    fn valid_point(p: Point, size: Size) -> bool {
        p.x.is_finite()
            && p.y.is_finite()
            && p.x >= 0.0
            && p.y >= 0.0
            && p.x < size.width
            && p.y < size.height
    }
}
impl Controller for Wda {
    type Error = WdaError;
    fn geometry(&mut self) -> Result<Size, WdaError> {
        self.geometry_cache = None;
        let response = self.session_request(Method::GET, "window/size", None)?;
        let width = response
            .pointer("/value/width")
            .and_then(Value::as_f64)
            .ok_or(WdaError::InvalidResponse)?;
        let height = response
            .pointer("/value/height")
            .and_then(Value::as_f64)
            .ok_or(WdaError::InvalidResponse)?;
        if width <= 0.0 || height <= 0.0 || width > 16384.0 || height > 16384.0 {
            return Err(WdaError::InvalidResponse);
        }
        let size = Size { width, height };
        self.geometry_cache = Some(size);
        Ok(size)
    }
    fn dispatch(&mut self, input: Input) -> Result<(), WdaError> {
        let tap = matches!(&input, Input::Tap(_));
        let start = Instant::now();
        let result = self.dispatch_input(input);
        if tap {
            if self.tap_durations.len() == 64 {
                self.tap_durations.pop_front();
            }
            self.tap_durations.push_back(start.elapsed());
            self.tap_failures += u64::from(result.is_err());
        }
        result
    }
}
impl Wda {
    fn dispatch_input(&mut self, input: Input) -> Result<(), WdaError> {
        let (path, body) = match input {
            Input::Tap(p) => {
                let size = self.cached_geometry()?;
                if !Self::valid_point(p, size) {
                    return Err(WdaError::InvalidInput);
                }
                ("wda/tap", json!({"x":p.x,"y":p.y}))
            }
            Input::Swipe { from, to, duration } => {
                let size = self.cached_geometry()?;
                if !Self::valid_point(from, size)
                    || !Self::valid_point(to, size)
                    || duration > Duration::from_secs(5)
                {
                    return Err(WdaError::InvalidInput);
                }
                (
                    "actions",
                    json!({"actions":[{"type":"pointer","id":"finger","parameters":{"pointerType":"touch"},
                    "actions":[{"type":"pointerMove","duration":0,"origin":"viewport","x":from.x,"y":from.y},
                    {"type":"pointerDown","button":0},
                    {"type":"pointerMove","duration":duration.as_millis() as u64,"origin":"viewport","x":to.x,"y":to.y},
                    {"type":"pointerUp","button":0}]}]}),
                )
            }
            Input::Text(text) => {
                if text.len() > 4096 {
                    return Err(WdaError::InvalidInput);
                }
                ("wda/keys", json!({"value":[text]}))
            }
            Input::Button(button) => match button {
                Button::Lock => ("wda/lock", json!({})),
                Button::Home => ("wda/pressButton", json!({"name":"home"})),
                Button::VolumeUp => ("wda/pressButton", json!({"name":"volumeup"})),
                Button::VolumeDown => ("wda/pressButton", json!({"name":"volumedown"})),
                Button::AppSwitcher => return Err(WdaError::InvalidInput),
            },
            Input::Release => {
                if self.session.is_some() {
                    self.session_request(Method::DELETE, "actions", None)?;
                }
                return Ok(());
            }
        };
        self.session_request(Method::POST, path, Some(&body))?;
        Ok(())
    }
}
#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_restrictions() {
        for address in [
            "http://192.168.1.2:8100",
            "https://127.0.0.1",
            "http://localhost:8100",
            "http://name@127.0.0.1:8100",
            "http://127.0.0.1/foo",
            "http://127.0.0.1/?token=x",
            "http://127.0.0.1/#fragment",
        ] {
            assert!(endpoint(address).is_err(), "{address}");
        }
        assert!(endpoint("http://127.0.0.1:8100").is_ok());
        assert!(endpoint("http://[::1]:8100").is_ok());
    }
    #[test]
    fn timeout_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let server = std::thread::spawn(move || {
            if let Ok((_socket, _)) = listener.accept() {
                std::thread::sleep(Duration::from_millis(350));
            }
        });
        let mut client = Wda::with_timeout(&address, Duration::from_millis(80))?;
        let start = Instant::now();
        assert!(matches!(client.status(), Err(WdaError::Connection(_))));
        assert!(start.elapsed() < Duration::from_secs(2));
        server.join().map_err(|_| "server panicked")?;
        Ok(())
    }
    #[test]
    fn oversized_response_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let server = std::thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut req = [0; 1024];
                let _ = socket.read(&mut req);
                let body = vec![b' '; MAX_REPLY as usize + 1];
                let _ = write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(&body);
            }
        });
        let mut client = Wda::new(&address)?;
        assert!(matches!(client.status(), Err(WdaError::InvalidResponse)));
        server.join().map_err(|_| "server panicked")?;
        Ok(())
    }
}
