//! Process-local diagnostics are shared by UI and input worker without UI-thread I/O.
use imirror_input_ble::TouchTrace;
use std::sync::OnceLock;
pub const BUILD_ID: &str = "direct-touch-one-click-20260911-01";

pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::args().any(|arg| arg == "--direct-touch-one-click"))
}
pub fn trace() -> &'static TouchTrace {
    static TRACE: OnceLock<TouchTrace> = OnceLock::new();
    TRACE.get_or_init(TouchTrace::default)
}
/// A new process must not overwrite the evidence from a previous failed attempt.
pub fn archive_path() -> Option<&'static std::path::Path> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    if !enabled() {
        return None;
    }
    PATH.get_or_init(|| {
        let base = crate::input::diagnostic_path()?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Some(base.with_file_name(format!(
            "direct-touch-click-{}-{stamp}.json",
            std::process::id()
        )))
    })
    .as_deref()
}
