//! One movement accumulator plus ordered, lossless button/key transitions.
//! No movement FIFO. Only explicit release/disconnect cancels pending transitions.
use crate::{
    PointerScale,
    metrics::{Metrics, now_ns},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
};

pub const MAX_TRANSITIONS: usize = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timing {
    pub received: u64,
    pub extracted: u64,
    pub queued: u64,
    pub latest: u64,
    pub button: u64,
    pub wheel: u64,
    pub motion_events: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Packet {
    Button {
        previous: u8,
        buttons: u8,
        dx: i32,
        dy: i32,
        wheel: i32,
        timing: Timing,
    },
    Mouse {
        buttons: u8,
        dx: i32,
        dy: i32,
        wheel: i32,
        timing: Timing,
    },
    Keyboard {
        modifiers: u8,
        keys: [u8; 6],
        timing: Timing,
    },
}
impl Packet {
    pub fn is_neutral(self) -> bool {
        match self {
            Self::Mouse {
                buttons,
                dx,
                dy,
                wheel,
                ..
            }
            | Self::Button {
                buttons,
                dx,
                dy,
                wheel,
                ..
            } => buttons == 0 && dx == 0 && dy == 0 && wheel == 0,
            Self::Keyboard {
                modifiers, keys, ..
            } => modifiers == 0 && keys == [0; 6],
        }
    }
    pub fn timing(self) -> Timing {
        match self {
            Self::Mouse { timing, .. }
            | Self::Keyboard { timing, .. }
            | Self::Button { timing, .. } => timing,
        }
    }
    fn expire_motion(&mut self, now: u64, age: u64, metrics: &Metrics) {
        if let Self::Mouse { dx, dy, timing, .. } | Self::Button { dx, dy, timing, .. } = self
            && timing.motion_events > 0
            && now.saturating_sub(timing.latest) > age
        {
            *dx = 0;
            *dy = 0;
            metrics
                .stale
                .fetch_add(timing.motion_events, Ordering::Relaxed);
            timing.motion_events = 0;
        }
    }
}
#[derive(Default)]
struct Motion {
    dx: i64,
    dy: i64,
    timing: Timing,
}
impl Motion {
    fn add(&mut self, dx: i32, dy: i32, received: u64, extracted: u64) {
        if self.timing.motion_events == 0 {
            self.timing.received = received;
            self.timing.extracted = extracted;
            self.timing.queued = now_ns();
        }
        self.dx = self.dx.saturating_add(i64::from(dx));
        self.dy = self.dy.saturating_add(i64::from(dy));
        self.timing.latest = received;
        self.timing.motion_events += 1;
    }
    fn packet(self, buttons: u8) -> Packet {
        Packet::Mouse {
            buttons,
            dx: self.dx.clamp(-32767, 32767) as i32,
            dy: self.dy.clamp(-32767, 32767) as i32,
            wheel: 0,
            timing: self.timing,
        }
    }
}
struct State {
    motion: Motion,
    transitions: VecDeque<Packet>,
    buttons: u8,
    modifiers: u8,
    keys: [u8; 6],
    gain: PointerScale,
    wheel_remainder: i32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            motion: Motion::default(),
            transitions: VecDeque::with_capacity(MAX_TRANSITIONS),
            buttons: 0,
            modifiers: 0,
            keys: [0; 6],
            gain: PointerScale::default(),
            wheel_remainder: 0,
        }
    }
}
pub struct Mailbox {
    state: Mutex<State>,
    wake: Arc<dyn Fn() + Send + Sync>,
    pub metrics: Arc<Metrics>,
    pub captured: AtomicBool,
    pub ready: AtomicBool,
    pub enabled: AtomicBool,
    pub sensitivity: AtomicU16,
}
impl Mailbox {
    pub fn new(metrics: Arc<Metrics>, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            wake,
            metrics,
            captured: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            enabled: AtomicBool::new(false),
            sensitivity: AtomicU16::new(100),
        }
    }
    pub fn wake(&self) {
        (self.wake)();
    }
    pub fn capture(&self, initial_buttons: u8, at: u64) -> bool {
        if !self.enabled.load(Ordering::Acquire) || !self.ready.load(Ordering::Acquire) {
            return false;
        }
        self.captured.store(true, Ordering::Release);
        self.mouse(0, 0, initial_buttons, 0, at, at);
        true
    }
    pub fn release(&self) {
        // Local capture ends before any GATT work; callers immediately unconfine the cursor.
        self.captured.store(false, Ordering::Release);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        self.neutralize(&mut state);
        drop(state);
        self.wake();
    }
    /// Invalidate locally before the worker can finish any pending GATT operation.
    pub fn invalidate(&self) {
        self.ready.store(false, Ordering::Release);
        self.release();
    }
    fn neutralize(&self, state: &mut State) {
        self.metrics
            .stale
            .fetch_add(state.motion.timing.motion_events, Ordering::Relaxed);
        state.motion = Motion::default();
        state.transitions.clear();
        state.buttons = 0;
        state.keys = [0; 6];
        state.modifiers = 0;
        state.gain.reset();
        state.wheel_remainder = 0;
        // All-UP supersedes every queued release, including an UP whose DOWN
        // is already in flight. Reserved capacity makes emergency release infallible.
        {
            let at = now_ns();
            let timing = Timing {
                received: 0, // Safety neutral, not a physical input-latency sample.
                extracted: 0,
                queued: at,
                latest: at,
                ..Timing::default()
            };
            state.transitions.push_back(Packet::Mouse {
                buttons: 0,
                dx: 0,
                dy: 0,
                wheel: 0,
                timing,
            });
            state.transitions.push_back(Packet::Keyboard {
                modifiers: 0,
                keys: [0; 6],
                timing,
            });
        }
        self.metrics.pending(0);
    }
    fn overload(&self, state: &mut State) -> bool {
        if state.transitions.len() < MAX_TRANSITIONS {
            return false;
        }
        // Stop accepting presses. Never lose a release: collapse all outstanding
        // mouse/key UPs into two all-UP reports and return local capture immediately.
        self.captured.store(false, Ordering::Release);
        self.metrics.rejected.fetch_add(1, Ordering::Relaxed);
        self.neutralize(state);
        self.wake();
        true
    }
    pub fn transition_depth(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .transitions
            .len()
    }
    pub fn mouse(
        &self,
        dx: i32,
        dy: i32,
        buttons: u8,
        wheel_units: i32,
        received: u64,
        extracted: u64,
    ) {
        if !self.captured.load(Ordering::Acquire) {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !self.captured.load(Ordering::Acquire) {
            return;
        }
        let gain = self.sensitivity.load(Ordering::Relaxed);
        if state.gain.percent() != gain {
            state.gain.set_percent(gain);
        }
        let (dx, dy) = state.gain.scale(dx, dy);
        self.metrics
            .scale
            .record(now_ns().saturating_sub(extracted));
        let had_motion = state.motion.timing.motion_events > 0;
        if dx != 0 || dy != 0 {
            if had_motion {
                self.metrics.coalesced.fetch_add(1, Ordering::Relaxed);
            }
            state.motion.add(dx, dy, received, extracted);
        }
        state.wheel_remainder = state.wheel_remainder.saturating_add(wheel_units);
        let wheel = state.wheel_remainder / 120;
        state.wheel_remainder %= 120;
        let changed = buttons != state.buttons;
        if changed || wheel != 0 {
            if self.overload(&mut state) {
                return;
            }
            let motion = std::mem::take(&mut state.motion);
            let mut timing = motion.timing;
            if timing.received == 0 {
                timing = Timing {
                    received,
                    extracted,
                    queued: now_ns(),
                    latest: received,
                    ..Timing::default()
                };
            }
            if changed {
                timing.button = received;
            }
            if wheel != 0 {
                timing.wheel = received;
            }
            let dx = motion.dx.clamp(-32767, 32767) as i32;
            let dy = motion.dy.clamp(-32767, 32767) as i32;
            // A transition carries its pre-transition aggregate. The sender flushes
            // that movement with the PREVIOUS button state, then sends the transition.
            // This prevents the last drag segment from being applied after mouse-up.
            let previous = state.buttons;
            state.transitions.push_back(if changed {
                Packet::Button {
                    previous,
                    buttons,
                    dx,
                    dy,
                    wheel,
                    timing,
                }
            } else {
                Packet::Mouse {
                    buttons,
                    dx,
                    dy,
                    wheel,
                    timing,
                }
            });
            state.buttons = buttons;
            self.metrics.pending(0);
            drop(state);
            self.wake();
        } else {
            self.metrics
                .pending(u64::from(state.motion.timing.motion_events > 0));
            drop(state);
            if !had_motion && (dx != 0 || dy != 0) {
                self.wake();
            }
        }
    }
    pub fn key(&self, modifiers: u8, keys: [u8; 6], received: u64) {
        if !self.captured.load(Ordering::Acquire) {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !self.captured.load(Ordering::Acquire) {
            return;
        }
        if (state.modifiers, state.keys) == (modifiers, keys) {
            return;
        }
        if self.overload(&mut state) {
            return;
        }
        state.modifiers = modifiers;
        state.keys = keys;
        state.transitions.push_back(Packet::Keyboard {
            modifiers,
            keys,
            timing: Timing {
                received,
                extracted: received,
                queued: now_ns(),
                latest: received,
                ..Timing::default()
            },
        });
        drop(state);
        self.wake();
    }
    pub fn take(&self, now: u64, movement_due: bool, stale_age: u64) -> Option<Packet> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut packet) = state.transitions.pop_front() {
            packet.expire_motion(now, stale_age, &self.metrics);
            return Some(packet);
        }
        if !movement_due || state.motion.timing.motion_events == 0 {
            return None;
        }
        let motion = std::mem::take(&mut state.motion);
        let mut packet = motion.packet(state.buttons);
        self.metrics.pending(0);
        packet.expire_motion(now, stale_age, &self.metrics);
        if let Packet::Mouse { dx: 0, dy: 0, .. } = packet {
            return None;
        }
        Some(packet)
    }
    pub fn has_pending(&self) -> bool {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.motion.timing.motion_events > 0 || !state.transitions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overload_keeps_all_up_bounded_and_stops_new_presses() {
        let m = mailbox();
        for t in 0..10000 {
            m.mouse(0, 0, (t % 2) as u8, 120, t + 1, t + 1);
            m.key((t % 2) as u8, [4, 0, 0, 0, 0, 0], t + 1);
            assert!(m.transition_depth() <= MAX_TRANSITIONS);
        }
        assert!(!m.captured.load(Ordering::Acquire));
        assert!(matches!(
            m.take(20000, false, 100),
            Some(Packet::Mouse {
                buttons: 0,
                dx: 0,
                dy: 0,
                wheel: 0,
                ..
            })
        ));
        assert!(matches!(
            m.take(20001, false, 100),
            Some(Packet::Keyboard {
                modifiers: 0,
                keys: [0, 0, 0, 0, 0, 0],
                ..
            })
        ));
        assert!(m.take(20002, true, 100).is_none());
    }
    #[test]
    fn invalidation_clears_motion_even_with_worker_packet_in_flight() {
        let m = mailbox();
        m.mouse(4, 2, 1, 0, 2, 2);
        let _in_flight = m.take(3, false, 100);
        m.mouse(90, 30, 1, 0, 4, 4);
        m.invalidate();
        assert!(!m.ready.load(Ordering::Acquire));
        assert!(!m.capture(0, 5));
        assert_eq!(m.metrics.pending.load(Ordering::Relaxed), 0);
        assert!(matches!(
            m.take(6, false, 100),
            Some(Packet::Mouse {
                buttons: 0,
                dx: 0,
                dy: 0,
                ..
            })
        ));
    }
    #[test]
    fn taking_capture_does_not_hold_a_remote_button() {
        let m = mailbox();
        assert!(m.take(10, true, 100).is_none());
        m.mouse(20, 5, 0, 0, 11, 11);
        assert!(matches!(
            m.take(12, true, 100),
            Some(Packet::Mouse {
                buttons: 0,
                dx: 20,
                dy: 5,
                ..
            })
        ));
    }
    fn mailbox() -> Mailbox {
        let m = Mailbox::new(Arc::new(Metrics::default()), Arc::new(|| {}));
        m.ready.store(true, Ordering::Relaxed);
        m.enabled.store(true, Ordering::Relaxed);
        m.capture(0, 1);
        m
    }
    #[test]
    fn thousand_events_coalesce_to_one_packet_and_no_tail() {
        let m = mailbox();
        for t in 1..=1000 {
            m.mouse(1, -1, 0, 0, t, t);
        }
        assert_eq!(m.metrics.max_pending.load(Ordering::Relaxed), 1);
        assert!(matches!(
            m.take(1001, true, 10000),
            Some(Packet::Mouse {
                dx: 1000,
                dy: -1000,
                ..
            })
        ));
        assert!(m.take(1002, true, 10000).is_none());
        assert_eq!(m.metrics.coalesced.load(Ordering::Relaxed), 999);
    }
    #[test]
    fn button_transitions_preserve_order_without_movement_wait() {
        let m = mailbox();
        m.mouse(10, 0, 0, 0, 1, 1);
        m.mouse(0, 0, 1, 0, 2, 2);
        m.mouse(20, 0, 1, 0, 3, 3);
        m.mouse(0, 0, 0, 0, 4, 4);
        assert!(matches!(
            m.take(5, false, 100),
            Some(Packet::Button {
                buttons: 1,
                dx: 10,
                ..
            })
        ));
        assert!(matches!(
            m.take(6, false, 100),
            Some(Packet::Button {
                buttons: 0,
                dx: 20,
                ..
            })
        ));
        assert!(m.take(7, true, 100).is_none());
    }
    #[test]
    fn stale_motion_is_discarded_but_button_up_is_not() {
        let m = mailbox();
        m.mouse(50, 0, 1, 0, 1, 1);
        m.mouse(0, 0, 0, 0, 2, 2);
        assert!(matches!(
            m.take(100, false, 20),
            Some(Packet::Button {
                buttons: 1,
                dx: 0,
                ..
            })
        ));
        assert!(matches!(
            m.take(101, false, 20),
            Some(Packet::Button { buttons: 0, .. })
        ));
        assert_eq!(m.metrics.stale.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn wheel_bypasses_motion_pacing_and_release_neutralizes() {
        let m = mailbox();
        m.mouse(0, 0, 0, 60, 1, 1);
        assert!(m.take(2, false, 100).is_none());
        m.mouse(0, 0, 0, 60, 3, 3);
        assert!(matches!(
            m.take(4, false, 100),
            Some(Packet::Mouse { wheel: 1, .. })
        ));
        m.key(2, [4, 0, 0, 0, 0, 0], 5);
        m.release();
        assert!(!m.captured.load(Ordering::Relaxed));
        assert!(matches!(
            m.take(6, false, 100),
            Some(Packet::Mouse { buttons: 0, .. })
        ));
        assert!(matches!(
            m.take(7, false, 100),
            Some(Packet::Keyboard {
                modifiers: 0,
                keys: [0, 0, 0, 0, 0, 0],
                ..
            })
        ));
    }
}
