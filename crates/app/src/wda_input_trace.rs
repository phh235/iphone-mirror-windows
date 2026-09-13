//! Bounded click-path evidence, independent of BLE packet metrics and WDA sessions.
use imirror_coordinate_map::Point;
use std::sync::Mutex;

#[derive(Clone, Copy, Default)]
pub struct Origin {
    pub down_ns: u64,
    pub up_ns: u64,
    pub classified_ns: u64,
}
#[derive(Clone, Copy)]
pub struct Dispatch {
    pub origin: Option<Origin>,
    pub queued_ns: u64,
    pub dequeued_ns: u64,
    pub completed_ns: u64,
    pub http: Option<imirror_input_wda::timing::HttpTiming>,
    pub http_requests: u64,
    pub operation: &'static str,
    pub success: bool,
}

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
    origin: Option<Origin>,
    dispatches: [Option<Dispatch>; 128],
    dispatch_count: usize,
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
            origin: None,
            dispatches: [None; 128],
            dispatch_count: 0,
        }
    }
    fn push(&mut self, event: Event) {
        match event.stage {
            "native_down" => {
                self.origin = Some(Origin {
                    down_ns: event.at_ns,
                    ..Default::default()
                })
            }
            "native_up" => {
                if let Some(origin) = &mut self.origin {
                    origin.up_ns = event.at_ns;
                }
            }
            "classified" => {
                if let Some(origin) = &mut self.origin {
                    origin.classified_ns = event.at_ns;
                }
            }
            "cancelled" | "rejected" => self.origin = None,
            _ => {}
        }
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

pub fn take_origin() -> Option<Origin> {
    TRACE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .origin
        .take()
        .filter(|o| o.up_ns >= o.down_ns && o.classified_ns >= o.up_ns && o.classified_ns != 0)
}
pub fn dispatch(sample: Dispatch) {
    let mut trace = TRACE.lock().unwrap_or_else(|e| e.into_inner());
    let index = trace.dispatch_count % trace.dispatches.len();
    trace.dispatches[index] = Some(sample);
    trace.dispatch_count = trace.dispatch_count.wrapping_add(1);
}

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
    let count = trace.dispatch_count.min(trace.dispatches.len());
    let dispatches: Vec<_> = (trace.dispatch_count.saturating_sub(count)..trace.dispatch_count)
        .filter_map(|i| trace.dispatches[i % trace.dispatches.len()])
        .map(|d| {
            let ms = |a: u64, b: u64| b.saturating_sub(a) as f64 / 1e6;
            serde_json::json!({"operation":d.operation,"success":d.success,
                "mouse_down_ns":d.origin.map(|o|o.down_ns),"mouse_up_ns":d.origin.map(|o|o.up_ns),
                "classified_ns":d.origin.map(|o|o.classified_ns),"queued_ns":d.queued_ns,
                "dequeued_ns":d.dequeued_ns,"completed_ns":d.completed_ns,
                "mouse_down_to_enqueue_ms":d.origin.map(|o|ms(o.down_ns,d.queued_ns)),
                "mouse_up_to_enqueue_ms":d.origin.map(|o|ms(o.up_ns,d.queued_ns)),
                "queue_wait_ms":ms(d.queued_ns,d.dequeued_ns),
                "dispatch_ms":ms(d.dequeued_ns,d.completed_ns),
                "http_requests":d.http_requests,"last_http":d.http.map(|h|h.json()),
                "dispatch_to_http_ms":d.http.map(|h|ms(d.dequeued_ns,h.start_ns)),
                "input_to_visual_ms":null})
        })
        .collect();
    serde_json::json!({"native_down":trace.native_down,"native_up":trace.native_up,
        "queued":trace.queued,"dispatched":trace.dispatched,"failed":trace.failed,
        "events":events,"event_limit":trace.events.len(),"dispatches":dispatches,
        "dispatch_limit":trace.dispatches.len(),
        "dispatch_note":"Matched monotonic command observations. mouse_down includes human hold duration. Multiple-HTTP dispatches expose only the last HTTP breakdown. Visual response requires a separate frame observation.",
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
    #[test]
    fn gesture_origin_is_classified_and_cancelled_independently_of_old_dispatches() {
        let mut trace = Trace::new();
        for (stage, at_ns) in [("native_down", 10), ("native_up", 20), ("classified", 21)] {
            trace.push(Event {
                at_ns,
                stage,
                detail: "",
                point: None,
            });
        }
        assert_eq!(
            trace.origin.map(|o| (o.down_ns, o.up_ns, o.classified_ns)),
            Some((10, 20, 21))
        );
        trace.push(Event {
            at_ns: 22,
            stage: "completed",
            detail: "older queued request",
            point: None,
        });
        assert!(trace.origin.is_some());
        trace.push(Event {
            at_ns: 23,
            stage: "cancelled",
            detail: "focus loss",
            point: None,
        });
        assert!(trace.origin.is_none());
    }
}
