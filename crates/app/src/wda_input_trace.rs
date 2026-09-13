//! Bounded click-path evidence, independent of BLE packet metrics and WDA sessions.
use imirror_coordinate_map::Point;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct Event {
    at_ns: u64,
    stage: &'static str,
    detail: &'static str,
    point: Option<Point>,
}
#[derive(Clone)]
struct Trace {
    events: [Option<Event>; 32],
    total: usize,
    native_down: u64,
    native_up: u64,
    queued: u64,
    dispatched: u64,
    failed: u64,
}
impl Trace {
    const fn new() -> Self {
        Self {
            events: [None; 32],
            total: 0,
            native_down: 0,
            native_up: 0,
            queued: 0,
            dispatched: 0,
            failed: 0,
        }
    }
    fn push(&mut self, event: Event) {
        match event.stage {
            "native_down" => self.native_down += 1,
            "native_up" => self.native_up += 1,
            "queued" => self.queued += 1,
            "dispatch" => self.dispatched += 1,
            "failed" => self.failed += 1,
            _ => {}
        }
        self.events[self.total % self.events.len()] = Some(event);
        self.total = self.total.wrapping_add(1);
    }
}
static TRACE: Mutex<Trace> = Mutex::new(Trace::new());

pub fn record(stage: &'static str, detail: &'static str, point: Option<Point>) {
    TRACE.lock().unwrap_or_else(|e| e.into_inner()).push(Event {
        at_ns: imirror_input_core::metrics::now_ns(),
        stage,
        detail,
        point,
    });
}

pub fn snapshot() -> serde_json::Value {
    let trace = TRACE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let count = trace.total.min(trace.events.len());
    let start = trace.total.saturating_sub(count);
    let events: Vec<_> = (start..trace.total)
        .filter_map(|i| trace.events[i % trace.events.len()])
        .map(|e| {
            serde_json::json!({"at_ns":e.at_ns,"stage":e.stage,"detail":e.detail,
                "point":e.point.map(|p|[p.x,p.y])})
        })
        .collect();
    serde_json::json!({"native_down":trace.native_down,"native_up":trace.native_up,
        "queued":trace.queued,"dispatched":trace.dispatched,"failed":trace.failed,
        "events":events,"event_limit":trace.events.len(),
        "note":"Lifetime of this app process, not reset by WDA session recreation. Native counters observe preview clicks, including BLE capture initiation. Dispatch success is not physical acceptance. No typed text is recorded."})
}

pub fn gate(
    enabled: bool,
    wda: bool,
    ready: bool,
    video_live: bool,
    geometry: bool,
) -> Result<(), &'static str> {
    if !enabled {
        Err("Control is disabled")
    } else if !wda {
        Err("WDA backend is not selected or is restarting")
    } else if !ready {
        Err("WDA session is not ready")
    } else if !video_live {
        Err("The UI has no live video format")
    } else if !geometry {
        Err("The UI has no WDA device geometry")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_does_not_bypass_video_or_geometry_gate() {
        assert_eq!(
            gate(true, true, true, true, false),
            Err("The UI has no WDA device geometry")
        );
        assert_eq!(
            gate(true, true, true, false, true),
            Err("The UI has no live video format")
        );
        assert!(gate(false, true, true, true, true).is_err());
        assert!(gate(true, false, true, true, true).is_err());
        assert!(gate(true, true, false, true, true).is_err());
        assert!(gate(true, true, true, true, true).is_ok());
    }
    #[test]
    fn events_are_bounded_and_reconnect_cannot_erase_failure_count() {
        let mut trace = Trace::new();
        for i in 0..100 {
            trace.push(Event {
                at_ns: i,
                stage: "failed",
                detail: "request failed",
                point: None,
            });
        }
        trace.push(Event {
            at_ns: 101,
            stage: "session_ready",
            detail: "new session",
            point: None,
        });
        assert_eq!(trace.failed, 100);
        assert_eq!(trace.events.iter().flatten().count(), 32);
        assert_eq!(
            trace.events[100 % 32].map(|e| e.stage),
            Some("session_ready")
        );
    }
}
