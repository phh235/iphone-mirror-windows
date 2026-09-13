//! Explicit developer fixture API, excluded from ordinary builds. No automatic input.
use super::*;

impl Wda {
    /// Reuse the GUI's session without replacing it or changing server settings.
    pub fn probe_adopt_running_session(&mut self) -> Result<(), WdaError> {
        self.borrowed_session = true;
        let status = self.status()?;
        let id = status["sessionId"]
            .as_str()
            .ok_or(WdaError::SessionExpired)?;
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(WdaError::InvalidResponse);
        }
        self.session = Some(id.to_owned());
        let settings = self.session_request(Method::GET, "appium/settings", None)?;
        self.fast_tap = settings["value"]["waitForIdleTimeout"].as_f64() == Some(0.0)
            && settings["value"]["animationCoolOffTimeout"].as_f64() == Some(0.0);
        if !self.fast_tap {
            return Err(WdaError::InvalidInput);
        }
        self.geometry()?;
        Ok(())
    }
    pub fn probe_active_app(&mut self) -> Result<Value, WdaError> {
        self.session_request(Method::GET, "wda/activeAppInfo", None)
    }
    pub fn probe_source(&mut self) -> Result<Value, WdaError> {
        self.session_request(Method::GET, "source?format=json", None)
    }
    pub fn probe_tap(&mut self, point: Point, contact_ms: u64) -> Result<(), WdaError> {
        if !matches!(contact_ms, 10 | 20 | 30 | 50) {
            return Err(WdaError::InvalidInput);
        }
        let size = self.cached_geometry()?;
        if !self.fast_tap || !Self::valid_point(point, size) {
            return Err(WdaError::InvalidInput);
        }
        self.session_request(
            Method::POST,
            "actions",
            Some(&json!({"actions":[{
            "type":"pointer","id":"imirror-tap","parameters":{"pointerType":"touch"},"actions":[
                {"type":"pointerMove","duration":0,"origin":"viewport","x":point.x,"y":point.y},
                {"type":"pointerDown","button":0},{"type":"pause","duration":contact_ms},
                {"type":"pointerUp","button":0}]}]})),
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: String,
    pub texts: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Node {
    pub fn result_display(&self) -> bool {
        self.texts
            .iter()
            .any(|s| s.starts_with("StandardInputView;value:"))
    }
    pub fn button(&self) -> bool {
        self.kind.ends_with("Button") || self.kind == "Key" || self.kind == "XCUIElementTypeKey"
    }
    pub fn has(&self, text: &str) -> bool {
        self.texts
            .iter()
            .any(|s| s == text || s.strip_prefix("StandardInputView;value:") == Some(text))
    }
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.,
            y: self.y + self.height / 2.,
        }
    }
}

pub fn nodes(value: &Value) -> Result<Vec<Node>, WdaError> {
    fn visit(value: &Value, depth: usize, output: &mut Vec<Node>) -> Result<(), WdaError> {
        if depth > 64 || output.len() > 4096 {
            return Err(WdaError::InvalidResponse);
        }
        if let Some(object) = value.as_object() {
            let rect = object.get("rect").or_else(|| object.get("frame"));
            if let Some(rect) = rect {
                let dims = ["x", "y", "width", "height"].map(|k| rect[k].as_f64());
                if let [Some(x), Some(y), Some(width), Some(height)] = dims
                    && [x, y, width, height].iter().all(|v| v.is_finite())
                    && width > 0.
                    && height > 0.
                {
                    let texts = ["name", "label", "value"]
                        .iter()
                        .filter_map(|k| object.get(*k))
                        .filter_map(Value::as_str)
                        .map(|s| s.trim().to_owned())
                        .collect();
                    output.push(Node {
                        kind: object
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        texts,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
            for (key, child) in object {
                if key == "children" || key == "value" || key == "tree" {
                    visit(child, depth + 1, output)?;
                }
            }
        } else if let Some(array) = value.as_array() {
            for child in array {
                visit(child, depth + 1, output)?;
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(value, 0, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn button_digit_is_not_a_result_display() -> Result<(), WdaError> {
        let tree = json!({"value":{"children":[
            {"type":"Key","name":"One","label":"1","rect":{"x":1,"y":30,"width":20,"height":20}},
            {"type":"Other","name":"StandardInputView;value:1","rect":{"x":1,"y":1,"width":20,"height":20}}
        ]}});
        let nodes = nodes(&tree)?;
        assert_eq!(
            nodes.iter().filter(|n| n.has("1") && !n.button()).count(),
            1
        );
        Ok(())
    }
    #[test]
    fn borrowed_session_never_recreates_the_guis_session() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            let (mut stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut request = Vec::new();
            let mut bytes = [0u8; 1024];
            while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = stream.read(&mut bytes)?;
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&bytes[..n]);
                if request.len() > 8192 {
                    return Err(std::io::Error::other("Oversized test request"));
                }
            }
            let body = r#"{"value":{"error":"invalid session id"}}"#;
            write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )?;
            Ok(())
        });
        let mut client = Wda::with_timeout(&address, Duration::from_millis(300))?;
        client.session = Some("borrowed".into());
        client.borrowed_session = true;
        assert!(matches!(client.geometry(), Err(WdaError::SessionExpired)));
        assert_eq!(client.request_count(), 1);
        server.join().map_err(|_| "Test server panicked")??;
        Ok(())
    }
}
