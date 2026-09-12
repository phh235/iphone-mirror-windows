//! Monotonic software latency only. Fixed-size atomics avoid packet logging/allocations.
use serde::Serialize;
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
const SAMPLES: usize = 2048;
const RATE_SAMPLES: usize = 8192;
pub fn now_ns() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH
        .get_or_init(Instant::now)
        .elapsed()
        .as_nanos()
        .min(u64::MAX as u128) as u64
        + 1
}
pub struct Samples<const N: usize> {
    values: [AtomicU64; N],
    count: AtomicU64,
    sum: AtomicU64,
}
impl<const N: usize> Default for Samples<N> {
    fn default() -> Self {
        Self {
            values: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }
}
impl<const N: usize> Samples<N> {
    pub fn record(&self, value: u64) {
        let index = self.count.fetch_add(1, Ordering::Relaxed);
        self.values[index as usize % N].store(value, Ordering::Release);
        self.sum.fetch_add(value, Ordering::Relaxed);
    }
    fn values(&self) -> Vec<u64> {
        let count = self.count.load(Ordering::Acquire).min(N as u64) as usize;
        self.values[..count]
            .iter()
            .map(|x| x.load(Ordering::Acquire))
            .collect()
    }
    fn rate(&self, now: u64) -> u64 {
        self.values()
            .iter()
            .filter(|&&v| v != 0 && v <= now && now - v < 1_000_000_000)
            .count() as u64
    }
    fn latency(&self) -> Latency {
        let count = self.count.load(Ordering::Acquire);
        if count == 0 {
            return Latency::default();
        }
        let mut values = self.values();
        values.sort_unstable();
        let percentile = |p: usize| {
            values[(values.len() * p)
                .div_ceil(100)
                .saturating_sub(1)
                .min(values.len() - 1)] as f64
                / 1000.0
        };
        Latency {
            samples: count,
            avg_us: Some(self.sum.load(Ordering::Relaxed) as f64 / count as f64 / 1000.0),
            p50_us: Some(percentile(50)),
            p95_us: Some(percentile(95)),
            p99_us: Some(percentile(99)),
            max_us: values.last().map(|v| *v as f64 / 1000.0),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Latency {
    pub samples: u64,
    pub avg_us: Option<f64>,
    pub p50_us: Option<f64>,
    pub p95_us: Option<f64>,
    pub p99_us: Option<f64>,
    pub max_us: Option<f64>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    pub input_events_per_sec: u64,
    pub hid_reports_per_sec: u64,
    pub input_events: u64,
    pub hid_reports: u64,
    pub input_to_submit: Latency,
    pub input_to_gatt_completion: Latency,
    pub button_to_submit: Latency,
    pub wheel_to_submit: Latency,
    pub input_to_extract: Latency,
    pub extract_to_scale: Latency,
    pub queue_wait: Latency,
    pub max_pending_movement: u64,
    pub pending_movement: u64,
    pub coalesced_events: u64,
    pub dropped_stale_movement_events: u64,
    pub queue_rejections: u64,
    pub gatt_failures: u64,
}
#[derive(Default)]
pub struct Metrics {
    pub input_times: Samples<RATE_SAMPLES>,
    pub report_times: Samples<RATE_SAMPLES>,
    pub submit: Samples<SAMPLES>,
    pub complete: Samples<SAMPLES>,
    pub buttons: Samples<SAMPLES>,
    pub wheel: Samples<SAMPLES>,
    pub extract: Samples<SAMPLES>,
    pub scale: Samples<SAMPLES>,
    pub queue: Samples<SAMPLES>,
    pub pending: AtomicU64,
    pub max_pending: AtomicU64,
    pub coalesced: AtomicU64,
    pub stale: AtomicU64,
    pub rejected: AtomicU64,
    pub failed: AtomicU64,
}
impl Metrics {
    pub fn pending(&self, n: u64) {
        self.pending.store(n, Ordering::Relaxed);
        self.max_pending.fetch_max(n, Ordering::Relaxed);
    }
    pub fn submitted(&self, received: u64, at: u64, button: bool, wheel: bool) {
        if received == 0 {
            return;
        }
        self.report_times.record(at);
        self.submit.record(at.saturating_sub(received));
        if button {
            self.buttons.record(at.saturating_sub(received));
        }
        if wheel {
            self.wheel.record(at.saturating_sub(received));
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let now = now_ns();
        Snapshot {
            input_events_per_sec: self.input_times.rate(now),
            hid_reports_per_sec: self.report_times.rate(now),
            input_events: self.input_times.count.load(Ordering::Relaxed),
            hid_reports: self.report_times.count.load(Ordering::Relaxed),
            input_to_submit: self.submit.latency(),
            input_to_gatt_completion: self.complete.latency(),
            button_to_submit: self.buttons.latency(),
            wheel_to_submit: self.wheel.latency(),
            input_to_extract: self.extract.latency(),
            extract_to_scale: self.scale.latency(),
            queue_wait: self.queue.latency(),
            max_pending_movement: self.max_pending.load(Ordering::Relaxed),
            pending_movement: self.pending.load(Ordering::Relaxed),
            coalesced_events: self.coalesced.load(Ordering::Relaxed),
            dropped_stale_movement_events: self.stale.load(Ordering::Relaxed),
            queue_rejections: self.rejected.load(Ordering::Relaxed),
            gatt_failures: self.failed.load(Ordering::Relaxed),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safety_neutral_reports_are_not_physical_input_samples() {
        let metrics = Metrics::default();
        metrics.submitted(0, 100_000, true, false);
        assert_eq!(metrics.snapshot().hid_reports, 0);
        assert_eq!(metrics.snapshot().button_to_submit.samples, 0);
        metrics.submitted(50_000, 100_000, true, false);
        assert_eq!(metrics.snapshot().input_to_submit.avg_us, Some(50.0));
    }
    #[test]
    fn quantiles_and_empty_measurements_are_honest() {
        let samples = Samples::<128>::default();
        assert!(samples.latency().p95_us.is_none());
        for i in 1..=100 {
            samples.record(i * 1000);
        }
        let s = samples.latency();
        assert_eq!(s.avg_us, Some(50.5));
        assert_eq!(s.p50_us, Some(50.0));
        assert_eq!(s.p95_us, Some(95.0));
        assert_eq!(s.p99_us, Some(99.0));
    }
    #[test]
    fn rates_expire_and_sample_storage_is_bounded() {
        let samples = Samples::<8>::default();
        for i in 1..=100 {
            samples.record(i * 1_000_000);
        }
        assert_eq!(samples.values().len(), 8);
        assert_eq!(samples.rate(101_000_000), 8);
        assert_eq!(samples.rate(2_000_000_000), 0);
    }
}
