//! Monotonic request observations. Headers-ready is not first-byte or XCTest time.
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpTiming {
    pub start_ns: u64,
    pub request_ready_ns: u64,
    pub headers_ready_ns: Option<u64>,
    pub body_ready_ns: Option<u64>,
    pub completed_ns: u64,
}

impl HttpTiming {
    pub fn json(self) -> Value {
        let ms = |start: u64, end: u64| end.saturating_sub(start) as f64 / 1e6;
        json!({"start_ns":self.start_ns,"request_ready_ns":self.request_ready_ns,
            "headers_ready_ns":self.headers_ready_ns,"body_ready_ns":self.body_ready_ns,
            "completed_ns":self.completed_ns,
            "request_preparation_ms":ms(self.start_ns,self.request_ready_ns.max(self.start_ns)),
            "send_to_headers_ms":self.headers_ready_ns.map(|t|ms(self.request_ready_ns,t)),
            "response_body_ms":self.headers_ready_ns.zip(self.body_ready_ns).map(|(a,b)|ms(a,b)),
            "response_validation_ms":self.body_ready_ns.map(|t|ms(t,self.completed_ns)),
            "total_ms":ms(self.start_ns,self.completed_ns),
            "note":"send_to_headers includes HTTP, forwarding, WDA routing and XCTest; not separately attributable. Headers-ready is not first response byte."})
    }
}
