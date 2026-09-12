//! Every cache belongs to one observed subscription/session epoch.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

pub(crate) struct Epoch {
    generation: AtomicU64,
    invalidated: Mutex<Arc<dyn Fn() + Send + Sync>>,
}
impl Default for Epoch {
    fn default() -> Self {
        Self {
            generation: AtomicU64::new(1),
            invalidated: Mutex::new(Arc::new(|| {})),
        }
    }
}
impl Epoch {
    pub fn current(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
    pub fn matches(&self, generation: u64) -> bool {
        self.current() == generation
    }
    pub fn set_handler(&self, handler: Arc<dyn Fn() + Send + Sync>) {
        *self.invalidated.lock().unwrap_or_else(|e| e.into_inner()) = handler;
    }
    pub fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let handler = self
            .invalidated
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        handler();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconnect_cannot_revive_a_previous_subscription_epoch() {
        let epoch = Epoch::default();
        let notifications = Arc::new(AtomicU64::new(0));
        let seen = notifications.clone();
        epoch.set_handler(Arc::new(move || {
            seen.fetch_add(1, Ordering::Relaxed);
        }));
        let old = epoch.current();
        epoch.invalidate(); // CCCD removal/disconnect
        assert!(!epoch.matches(old));
        let reconnect = epoch.current();
        epoch.invalidate(); // fresh subscription or protocol transition
        assert!(!epoch.matches(old));
        assert!(!epoch.matches(reconnect));
        assert_eq!(notifications.load(Ordering::Acquire), 2);
    }
}
