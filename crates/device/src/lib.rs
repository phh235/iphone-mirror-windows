use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("No iPhone is connected")]
    NotFound,
    #[error("Unlock the iPhone and tap Trust This Computer")]
    TrustRequired,
    #[error("The device transport is unavailable: {0}")]
    Transport(String),
}
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Settings are malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Settings were created by a newer iMirror version")]
    FutureVersion,
    #[error("Receiver name must be 1–63 characters without control characters")]
    InvalidName,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Quality {
    #[default]
    Auto,
    Quality,
    Balanced,
    LowLatency,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub receiver_name: String,
    pub quality: Quality,
    pub vsync: bool,
    pub reconnect: bool,
    pub one_to_one: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            receiver_name: "iMirror".into(),
            quality: Quality::Auto,
            vsync: true,
            reconnect: true,
            one_to_one: false,
        }
    }
}
impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self, ConfigError> {
        let mut config: Self = serde_json::from_slice(bytes)?;
        if config.version > 2 {
            return Err(ConfigError::FutureVersion);
        }
        // v1 lacked vsync/reconnect. Serde defaults preserve safe v2 behavior.
        config.version = 2;
        if config.receiver_name.is_empty()
            || config.receiver_name.chars().count() > 63
            || config.receiver_name.chars().any(char::is_control)
        {
            return Err(ConfigError::InvalidName);
        }
        Ok(config)
    }
}
#[derive(Debug, Default)]
pub struct Reconnect {
    attempt: u32,
    next: Option<Instant>,
    enabled: bool,
}
impl Reconnect {
    pub fn request(&mut self, now: Instant) {
        self.enabled = true;
        self.attempt = 0;
        self.next = Some(now);
    }
    pub fn disconnect(&mut self) {
        self.enabled = false;
        self.next = None;
        self.attempt = 0;
    }
    pub fn ready(&self, now: Instant) -> bool {
        self.enabled && self.next.is_some_and(|next| now >= next)
    }
    pub fn failed(&mut self, now: Instant) -> Duration {
        self.attempt = self.attempt.saturating_add(1);
        let delay = Duration::from_millis(
            250u64.saturating_mul(1 << self.attempt.saturating_sub(1).min(5)),
        );
        self.next = Some(now + delay);
        delay
    }
    pub fn connected(&mut self) {
        self.attempt = 0;
        self.next = None;
    }
    pub fn lost(&mut self, now: Instant) {
        if self.enabled {
            self.failed(now);
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconnect_50_cycles_and_user_stop() {
        let mut retry = Reconnect::default();
        let mut now = Instant::now();
        retry.request(now);
        for _ in 0..50 {
            assert!(retry.ready(now));
            retry.connected();
            assert!(!retry.ready(now));
            retry.lost(now);
            assert!(!retry.ready(now));
            now += Duration::from_millis(250);
        }
        retry.disconnect();
        assert!(!retry.ready(now + Duration::from_secs(100)));
        retry.lost(now);
        assert!(!retry.enabled());
    }
    #[test]
    fn backoff_bounded_without_busy_loop() {
        let mut r = Reconnect::default();
        let now = Instant::now();
        r.request(now);
        for _ in 0..1000 {
            let delay = r.failed(now);
            assert!((Duration::from_millis(250)..=Duration::from_secs(8)).contains(&delay));
            assert!(!r.ready(now));
        }
    }
    #[test]
    fn config_migrates_old_and_rejects_future() -> Result<(), ConfigError> {
        let config = Config::parse(br#"{"version":1,"receiver_name":"Office"}"#)?;
        assert_eq!(config.version, 2);
        assert!(config.vsync && config.reconnect);
        assert!(matches!(
            Config::parse(br#"{"version":20}"#),
            Err(ConfigError::FutureVersion)
        ));
        assert!(Config::parse(br#"{"receiver_name":"bad\nname"}"#).is_err());
        Ok(())
    }
}
