//! Bounded, shared trace for one explicitly requested physical touch experiment.
//! Never writes files or waits for Bluetooth while holding the trace lock.
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize)]
pub struct TouchEvent {
    pub sequence: u64,
    pub unix_ms: u128,
    pub stage: String,
    pub detail: String,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct TouchTraceSnapshot {
    pub click_id: u64,
    pub events: VecDeque<TouchEvent>,
    pub omitted_events: u64,
}
#[derive(Clone, Default)]
pub struct TouchTrace(Arc<Mutex<TouchTraceSnapshot>>);
impl TouchTrace {
    pub fn snapshot(&self) -> TouchTraceSnapshot {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// One attempt per diagnostic process. A failed attempt is retained, never replayed.
    pub fn begin(&self, coordinates: String) -> bool {
        let mut trace = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if trace.click_id != 0 {
            trace.push(
                "EXTRA_DOWN_BLOCKED",
                "One-click test already consumed; no new contact sent.".into(),
            );
            return false;
        }
        trace.click_id = 1;
        trace.push("WINDOWS_MOUSE_DOWN", coordinates);
        true
    }
    pub fn record(&self, stage: &str, detail: impl Into<String>) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(stage, detail.into());
    }
    pub fn started(&self) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).click_id != 0
    }
}
impl TouchTraceSnapshot {
    fn push(&mut self, stage: &str, detail: String) {
        // Preserve the beginning (including rejection reasons) if unexpected events flood in.
        if self.events.len() >= 128 {
            self.omitted_events = self.omitted_events.saturating_add(1);
            return;
        }
        self.events.push_back(TouchEvent {
            sequence: self.events.len() as u64 + 1,
            unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            stage: stage.into(),
            detail,
        });
    }
    pub fn text(&self) -> String {
        if self.click_id == 0 {
            return "Mouse DOWN received: NO — waiting for one real click.".into();
        }
        let events = self
            .events
            .iter()
            .map(|e| format!("{} [{}] {}: {}", e.sequence, e.unix_ms, e.stage, e.detail))
            .collect::<Vec<_>>()
            .join("\r\n");
        format!(
            "Click ID: {} | omitted events: {}\r\n{}",
            self.click_id, self.omitted_events, events
        )
    }
}
pub fn report_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_click_cannot_be_replayed_and_first_evidence_survives_flood() {
        let trace = TouchTrace::default();
        assert!(trace.begin("(100,200)".into()));
        assert!(!trace.begin("(200,300)".into()));
        for _ in 0..200 {
            trace.record("NOISE", "ignored");
        }
        let state = trace.snapshot();
        assert_eq!(state.events.len(), 128);
        assert_eq!(state.events[0].stage, "WINDOWS_MOUSE_DOWN");
        assert_eq!(state.click_id, 1);
        assert!(state.omitted_events > 0);
    }
}
