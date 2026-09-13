//! Bounded diagnostic storage and an explicit, identifier-free clipboard export.
use crate::{control::Snapshot, worker};
use crossbeam_channel::Receiver;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const ARCHIVES: usize = 3;
pub struct Store {
    pub host: Mutex<Value>,
    pub error: Mutex<Option<String>>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            host: Mutex::new(Value::Null),
            error: Mutex::new(None),
        }
    }
}
struct RotatingLog {
    path: PathBuf,
    file: Option<File>,
    bytes: u64,
    limit: u64,
}
impl RotatingLog {
    fn open(path: PathBuf, limit: u64) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let bytes = file.metadata()?.len();
        let mut log = Self {
            path,
            file: Some(file),
            bytes,
            limit,
        };
        if bytes > limit {
            log.rotate()?;
        }
        Ok(log)
    }
    fn archive(&self, index: usize) -> PathBuf {
        let mut path = self.path.as_os_str().to_os_string();
        path.push(format!(".{index}"));
        PathBuf::from(path)
    }
    fn rotate(&mut self) -> io::Result<()> {
        self.file.take();
        for i in (1..=ARCHIVES).rev() {
            let to = self.archive(i);
            if to.exists() {
                fs::remove_file(&to)?;
            }
            let from = if i == 1 {
                self.path.clone()
            } else {
                self.archive(i - 1)
            };
            if from.exists() {
                // Historical unbounded logs are not retained above the new cap.
                if fs::metadata(&from)?.len() <= self.limit {
                    fs::rename(from, to)?;
                } else {
                    fs::remove_file(from)?;
                }
            }
        }
        self.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        self.bytes = 0;
        Ok(())
    }
    fn write(&mut self, line: &[u8]) -> io::Result<()> {
        let size = line.len() as u64 + 1;
        if size > self.limit {
            return Err(io::Error::other(
                "Diagnostic record exceeds bounded log size",
            ));
        }
        if self.file.is_none() || self.bytes + size > self.limit {
            self.rotate()?;
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("Diagnostic log unavailable"))?;
        file.write_all(line)?;
        file.write_all(b"\n")?;
        file.flush()?;
        self.bytes += size;
        Ok(())
    }
}
pub fn start_writer(receiver: Receiver<Snapshot>) -> io::Result<(JoinHandle<()>, Arc<Store>)> {
    let store = Arc::new(Store::default());
    let output = store.clone();
    let handle = thread::Builder::new()
        .name("imirror-control-diagnostics".into())
        .spawn(move || {
            let host = imirror_platform_windows::detect()
                .map(|h| json!({"windows_build":h.windows_build,"cpu":h.cpu,"gpus":h.gpus}));
            match host {
                Ok(host) => *output.host.lock().unwrap_or_else(|e| e.into_inner()) = host,
                Err(e) => {
                    *output.error.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(sanitize(&e.to_string()))
                }
            }
            let path = crate::control::diagnostic_path();
            let mut log = None;
            for snapshot in receiver {
                let result = (|| -> io::Result<()> {
                    let path = path.as_ref().ok_or_else(|| {
                        io::Error::other("Local diagnostic directory unavailable")
                    })?;
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    if log.is_none() {
                        log = Some(RotatingLog::open(
                            path.with_extension("samples.jsonl"),
                            MAX_LOG_BYTES,
                        )?);
                    }
                    let value = control_value(&snapshot);
                    let bytes = serde_json::to_vec(&value)?;
                    fs::write(path, serde_json::to_vec_pretty(&value)?)?;
                    if let Some(log) = &mut log {
                        log.write(&bytes)?;
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    *output.error.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(sanitize(&error.to_string()));
                }
            }
        })?;
    Ok((handle, store))
}
fn control_value(input: &Snapshot) -> Value {
    let b = &input.ble;
    json!({"app_version":env!("CARGO_PKG_VERSION"),"process_id":std::process::id(),"monotonic_ns":imirror_input_core::metrics::now_ns(),
        "input_source":"WM_INPUT / Raw Input","backend":input.mode,"ready":input.ready,"captured":input.captured,
        "pointer_speed_percent":input.pointer_speed_percent,"performance":input.performance,"transition_queue_depth":input.transition_queue_depth,
        "transition_queue_limit":imirror_input_core::relative::MAX_TRANSITIONS,"connection_interval_us":input.connection_interval_us,"pacing_interval_us":input.pacing_interval_us,
        "emergency_shortcut_available":input.emergency_shortcut_available,
        "wda":input.wda_metrics,
        "wda_runtime":input.wda_runtime,
        "wda_last_failure":input.wda_last_failure,
        "wda_click_path":crate::wda_input_trace::snapshot(),
        "wda_ui_geometry":input.geometry.map(|s|[s.width,s.height]),
        "bluetooth":{"peripheral_supported":b.adapter.as_ref().map(|a|a.peripheral),"low_energy_supported":b.adapter.as_ref().map(|a|a.low_energy),
            "radio":b.adapter.as_ref().map(|a|a.radio_state.as_str()),"hid_service_created":b.hid_service_created,"advertising":b.advertising_status,
            "mouse_subscribers":b.mouse_subscribers.len(),"keyboard_subscribers":b.keyboard_subscribers.len(),
            "selected_mouse_subscribed":b.selected_target.as_ref().is_some_and(|id|b.mouse_subscribers.contains(id)),
            "selected_keyboard_subscribed":b.selected_target.as_ref().is_some_and(|id|b.keyboard_subscribers.contains(id)),
            "protocol_mode":b.enumeration.protocol_mode,"suspended":b.enumeration.suspended},
        "error":input.ble_error.as_deref().map(sanitize)})
}
pub fn remember(errors: &mut VecDeque<String>, message: &str) {
    if message.is_empty() {
        return;
    }
    let message = sanitize(message);
    if errors.back() == Some(&message) {
        return;
    }
    if errors.len() == 16 {
        errors.pop_front();
    }
    errors.push_back(message);
}
fn sanitize(message: &str) -> String {
    message
        .split_whitespace()
        .take(128)
        .map(|word| {
            let hex = word.chars().filter(|c| c.is_ascii_hexdigit()).count();
            if word.contains("BluetoothLE#")
                || word.contains("\\\\?\\")
                || word.contains(":\\")
                || (hex >= 12
                    && word
                        .chars()
                        .all(|c| c.is_ascii_hexdigit() || "-:#".contains(c)))
            {
                "[identifier omitted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn report(
    input: &Snapshot,
    video: &worker::Snapshot,
    store: &Store,
    errors: &VecDeque<String>,
) -> Value {
    let mut report = control_value(input);
    report["host"] = store.host.lock().unwrap_or_else(|e| e.into_inner()).clone();
    // Deliberately exclude phone name, UDID, Bluetooth addresses, input text and raw event history.
    report["devices"] = json!(
        video
            .devices
            .iter()
            .map(|d| json!({"model":d.model,"ios":d.ios,"connection":d.connection}))
            .collect::<Vec<_>>()
    );
    report["video"] = json!({"state":video.status.state,"active":video.active,"decoder":video.decoder_mode,
        "width":video.status.width,"height":video.status.height,"source_fps":video.status.source_fps,
        "source_frames":video.status.source_frames,"last_decode_ms":video.status.last_decode_ms,"error":sanitize(&video.error)});
    report["recent_errors"] = json!(errors);
    report["diagnostics_error"] = json!(*store.error.lock().unwrap_or_else(|e| e.into_inner()));
    report["latency_note"] =
        json!("Software timestamps only; not physical iPhone or radio latency.");
    report
}
pub fn copy(
    window: windows::Win32::Foundation::HWND,
    value: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::{
        Foundation::{GlobalFree, HANDLE},
        System::{DataExchange::*, Memory::*},
    };
    let text: Vec<u16> = serde_json::to_string_pretty(value)?
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: The movable allocation contains a terminated UTF-16 string. Ownership
    // transfers to the clipboard only on success; every other path frees it.
    unsafe {
        let allocation = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2)?;
        let pointer = GlobalLock(allocation).cast::<u16>();
        if pointer.is_null() {
            let _ = GlobalFree(Some(allocation));
            return Err(windows::core::Error::from_win32().into());
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), pointer, text.len());
        let _ = GlobalUnlock(allocation);
        if let Err(error) = OpenClipboard(Some(window)) {
            let _ = GlobalFree(Some(allocation));
            return Err(error.into());
        }
        let result =
            EmptyClipboard().and_then(|_| SetClipboardData(13, Some(HANDLE(allocation.0))));
        let _ = CloseClipboard();
        if let Err(error) = result {
            let _ = GlobalFree(Some(allocation));
            return Err(error.into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_creates_parent_and_caps_every_file() -> io::Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "imirror-logs-{:?}",
            windows::core::GUID::new().map_err(io::Error::other)?
        ));
        let path = dir.join("nested/samples.jsonl");
        let mut log = RotatingLog::open(path.clone(), 128)?;
        for _ in 0..1000 {
            log.write(b"bounded diagnostic record")?;
        }
        drop(log);
        let files = fs::read_dir(path.parent().ok_or_else(|| io::Error::other("parent"))?)?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(files.len(), ARCHIVES + 1);
        for file in files {
            assert!(file.metadata()?.len() <= 128);
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test]
    fn export_omits_phone_and_bluetooth_identifiers() {
        let mut input = Snapshot::default();
        input.ble.selected_target = Some("secret-peer".into());
        input.ble.mouse_subscribers.push("secret-peer".into());
        let mut video = worker::Snapshot::default();
        video.devices.push(imirror_native_core::Device {
            id: "secret-udid".into(),
            name: "private-phone-name".into(),
            model: "iPhone15,4".into(),
            ios: "27.0".into(),
            connection: "USB".into(),
            status: String::new(),
            paired: true,
        });
        let text = report(&input, &video, &Store::default(), &VecDeque::new()).to_string();
        for secret in ["secret-peer", "secret-udid", "private-phone-name"] {
            assert!(!text.contains(secret));
        }
        assert!(text.contains("iPhone15,4"));
    }
}
