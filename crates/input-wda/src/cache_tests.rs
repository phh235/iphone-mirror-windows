use super::*;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    thread,
};

struct Step {
    line: &'static str,
    code: u16,
    body: Value,
    delay: Duration,
    expected_request: Option<Value>,
}
fn step(line: &'static str, body: Value) -> Step {
    Step {
        line,
        code: 200,
        body,
        delay: Duration::ZERO,
        expected_request: None,
    }
}
fn session(id: &str) -> Step {
    step("POST /session HTTP/1.1", json!({"sessionId":id,"value":{}}))
}
fn geometry(id: u8, width: u32, height: u32) -> Step {
    step(
        if id == 1 {
            "GET /session/one/window/size HTTP/1.1"
        } else {
            "GET /session/two/window/size HTTP/1.1"
        },
        json!({"value":{"width":width,"height":height}}),
    )
}
fn settings(id: u8) -> Step {
    Step {
        expected_request: Some(
            json!({"settings":{"waitForIdleTimeout":0,"animationCoolOffTimeout":0}}),
        ),
        ..step(
            if id == 1 {
                "POST /session/one/appium/settings HTTP/1.1"
            } else {
                "POST /session/two/appium/settings HTTP/1.1"
            },
            json!({"value":{"waitForIdleTimeout":0,"animationCoolOffTimeout":0}}),
        )
    }
}
fn tap(id: u8) -> Step {
    step(
        if id == 1 {
            "POST /session/one/actions HTTP/1.1"
        } else {
            "POST /session/two/actions HTTP/1.1"
        },
        json!({"value":null}),
    )
}
fn expired() -> Step {
    Step {
        code: 404,
        body: json!({"value":{"error":"invalid session id"}}),
        ..tap(1)
    }
}
type MockServer = (String, thread::JoinHandle<Result<(), String>>);
fn server(steps: Vec<Step>) -> Result<MockServer, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = format!("http://{}", listener.local_addr()?);
    let handle = thread::spawn(move || {
        for expected in steps {
            let deadline = Instant::now() + Duration::from_secs(4);
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => return Err(format!("Missing request {}: {e}", expected.line)),
                }
            };
            socket.set_nonblocking(false).map_err(|e| e.to_string())?;
            socket
                .set_read_timeout(Some(Duration::from_secs(1)))
                .map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(&mut socket);
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            if line.trim() != expected.line {
                return Err(format!("Expected {}, got {}", expected.line, line.trim()));
            }
            let mut length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).map_err(|e| e.to_string())?;
                if line == "\r\n" {
                    break;
                }
                if line.is_empty() {
                    return Err("Truncated request headers".into());
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse::<usize>().map_err(|e| e.to_string())?;
                }
            }
            if length > 8192 {
                return Err("Unexpected request size".into());
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).map_err(|e| e.to_string())?;
            if let Some(expected_body) = expected.expected_request {
                let actual: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
                if actual != expected_body {
                    return Err(format!("Wrong request body for {}", expected.line));
                }
            }
            drop(reader);
            thread::sleep(expected.delay);
            let reply = expected.body.to_string();
            let result = write!(
                socket,
                "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                expected.code,
                reply.len(),
                reply
            );
            if expected.delay.is_zero() {
                result.map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    });
    Ok((address, handle))
}
fn join(handle: thread::JoinHandle<Result<(), String>>) -> Result<(), Box<dyn std::error::Error>> {
    handle
        .join()
        .map_err(|_| "Mock server panicked")?
        .map_err(Into::into)
}
fn click() -> Input {
    Input::Tap(Point { x: 58.0, y: 656.0 })
}

#[test]
fn warm_taps_use_one_http_request_each() -> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        tap(1),
        tap(1),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    client.dispatch(click())?;
    client.dispatch(click())?;
    assert_eq!(client.geometry_requests, 1);
    assert_eq!(client.tap_requests, 2);
    assert_eq!(client.requests, 5);
    join(handle)
}
#[test]
fn explicit_rotation_refresh_replaces_cached_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        geometry(1, 852, 393),
        tap(1),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    client.geometry()?;
    client.dispatch(Input::Tap(Point { x: 800.0, y: 200.0 }))?;
    assert_eq!(client.geometry_requests, 2);
    join(handle)
}
#[test]
fn definite_session_rejection_refreshes_before_safe_retry() -> Result<(), Box<dyn std::error::Error>>
{
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        expired(),
        session("two"),
        settings(2),
        geometry(2, 393, 852),
        tap(2),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    client.dispatch(click())?;
    assert_eq!(client.geometry_requests, 2);
    assert_eq!(client.tap_requests, 2);
    join(handle)
}
#[test]
fn changed_geometry_never_replays_old_point() -> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        expired(),
        session("two"),
        settings(2),
        geometry(2, 852, 393),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    assert!(matches!(
        client.dispatch(click()),
        Err(WdaError::GeometryChanged)
    ));
    assert_eq!(client.tap_requests, 1);
    join(handle)
}
#[test]
fn ambiguous_tap_timeout_is_not_retried_and_invalidates_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        Step {
            delay: Duration::from_millis(200),
            ..tap(1)
        },
    ])?;
    let mut client = Wda::with_timeout(&address, Duration::from_millis(80))?;
    client.geometry()?;
    assert!(matches!(
        client.dispatch(click()),
        Err(WdaError::Connection(_))
    ));
    assert_eq!(client.tap_requests, 1);
    assert!(client.geometry_cache.is_none());
    join(handle)
}
#[test]
fn tap_metrics_remain_bounded_and_omit_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut steps = vec![session("one"), settings(1), geometry(1, 393, 852)];
    steps.extend((0..70).map(|_| tap(1)));
    let (address, handle) = server(steps)?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    for _ in 0..70 {
        client.dispatch(click())?;
    }
    let metrics = client.diagnostics();
    assert_eq!(client.tap_durations.len(), 64);
    assert_eq!(metrics["geometry_requests"], 1);
    assert_eq!(metrics["tap_requests"], 70);
    for key in ["session", "session_id", "x", "y", "text", "body"] {
        assert!(metrics.get(key).is_none());
    }
    join(handle)
}

#[test]
fn fast_tap_uses_single_complete_contact_without_geometry_query()
-> Result<(), Box<dyn std::error::Error>> {
    let expected = json!({"actions":[{"type":"pointer","id":"imirror-tap","parameters":{"pointerType":"touch"},"actions":[
        {"type":"pointerMove","duration":0,"origin":"viewport","x":58.0,"y":656.0},
        {"type":"pointerDown","button":0},{"type":"pause","duration":50},{"type":"pointerUp","button":0}
    ]}]});
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        Step {
            expected_request: Some(expected),
            ..tap(1)
        },
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    client.dispatch(click())?;
    assert!(client.fast_tap);
    assert_eq!(client.geometry_requests, 1);
    join(handle)
}
#[test]
fn unsupported_tuning_keeps_native_tap_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        Step {
            code: 404,
            body: json!({"value":{"error":"unknown command"}}),
            ..settings(1)
        },
        geometry(1, 393, 852),
        step("POST /session/one/wda/tap HTTP/1.1", json!({"value":null})),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    client.dispatch(click())?;
    assert!(!client.fast_tap);
    assert_eq!(client.tap_requests, 1);
    join(handle)
}

#[test]
fn changed_tap_mode_after_expiry_does_not_replay_old_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        expired(),
        session("two"),
        Step {
            code: 404,
            body: json!({"value":{"error":"unknown command"}}),
            ..settings(2)
        },
        geometry(2, 393, 852),
    ])?;
    let mut client = Wda::new(&address)?;
    client.geometry()?;
    assert!(matches!(
        client.dispatch(click()),
        Err(WdaError::TapModeChanged)
    ));
    assert_eq!(client.tap_requests, 1);
    join(handle)
}
#[test]
fn gesture_duration_extends_only_that_requests_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (address, handle) = server(vec![
        session("one"),
        settings(1),
        geometry(1, 393, 852),
        Step {
            delay: Duration::from_millis(200),
            ..tap(1)
        },
    ])?;
    let mut client = Wda::with_timeout(&address, Duration::from_millis(80))?;
    client.geometry()?;
    client.dispatch(Input::Swipe {
        from: Point { x: 10.0, y: 20.0 },
        to: Point { x: 50.0, y: 100.0 },
        duration: Duration::from_millis(250),
    })?;
    assert_eq!(client.tap_requests, 0);
    join(handle)
}
