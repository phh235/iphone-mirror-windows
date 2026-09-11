//! Owned, version-checked boundary to the vendored native media engine.
use serde::Serialize;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
static ENGINE_ACTIVE: AtomicBool = AtomicBool::new(false);

const VERSION: u32 = 18;

#[derive(Debug, thiserror::Error)]
#[error("Native media error {code}: {message}")]
pub struct NativeError {
    pub code: i32,
    pub message: String,
}

#[repr(C)]
#[derive(Clone)]
struct DeviceInfo {
    size: u32,
    version: u32,
    id: u32,
    port: u32,
    state: i32,
    usb: i32,
    pair: i32,
    lockdown: i32,
    udid: [u16; 128],
    name: [u16; 128],
    product: [u16; 64],
    os: [u16; 32],
    connection: [u16; 32],
    status: [u16; 192],
}

#[derive(Clone, Debug, Serialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub model: String,
    pub ios: String,
    pub connection: String,
    pub status: String,
    pub paired: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CaptureOptions {
    size: u32,
    version: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub audio: i32,
    pub volume: f32,
    reserved: [u32; 5],
}
impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            size: size_of::<Self>() as u32,
            version: VERSION,
            width: 0,
            height: 0,
            fps: 0,
            audio: 0,
            volume: 1.0,
            reserved: [0; 5],
        }
    }
}

#[repr(C)]
struct RawStatus {
    size: u32,
    version: u32,
    state: i32,
    width: u32,
    height: u32,
    fps: f64,
    latency_ms: f64,
    frames: u64,
    audio_packets: u64,
    audio_rate: u32,
    audio_channels: u32,
    failure_kind: i32,
    failure_stage: i32,
    error: i32,
    message: [u16; 192],
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub state: i32,
    pub width: u32,
    pub height: u32,
    pub source_fps: Option<f64>,
    pub last_decode_ms: f64,
    pub source_frames: u64,
    pub error: i32,
    pub failure_kind: i32,
    pub failure_stage: i32,
    pub message: String,
}

#[repr(C)]
#[derive(Default)]
struct RawOutputStatus {
    size: u32,
    version: u32,
    monitor_hdr: u32,
    source_hdr_known: i32,
    source_hdr: i32,
    hdr_surface: i32,
    hdr_effective: i32,
    requested_color: u32,
    requested_decoder: u32,
    applied_decoder: u32,
    switch_state: u32,
    runtime_mode: u32,
    requested_generation: u64,
    applied_generation: u64,
}

unsafe extern "C" {
    fn im_session_get_render_submissions(handle: u64, hwnd: *mut c_void, count: *mut u64) -> i32;
    fn im_session_set_view_preferences(
        handle: u64,
        hwnd: *mut c_void,
        one_to_one: i32,
        vsync: i32,
    ) -> i32;
    fn im_session_set_video_preferences(handle: u64, width: u32, height: u32, fps: u32) -> i32;
    fn im_session_get_video_output_status(
        handle: u64,
        hwnd: *mut c_void,
        output: *mut RawOutputStatus,
    ) -> i32;
    fn im_encoded_session_create(options: *const CaptureOptions, handle: *mut u64) -> i32;
    fn im_encoded_session_submit(
        handle: u64,
        avcc: *const u8,
        length: u32,
        sps: *const u8,
        sps_length: u32,
        pps: *const u8,
        pps_length: u32,
        width: u32,
        height: u32,
        pts: i64,
        keyframe: i32,
        discontinuity: i32,
    ) -> i32;
    fn im_encoded_session_audio(
        handle: u64,
        pcm: *const u8,
        length: u32,
        rate: u32,
        channels: u32,
    ) -> i32;
    fn im_encoded_session_reset(handle: u64) -> i32;

    fn im_initialize() -> i32;
    fn im_shutdown();
    fn im_api_version() -> u32;
    fn im_refresh_devices_ex(devices: *mut DeviceInfo, count: *mut u32, metadata: i32) -> i32;
    fn im_last_error() -> *const u16;
    fn im_session_create(id: *const u16, options: *const CaptureOptions, handle: *mut u64) -> i32;
    fn im_wireless_session_create(
        id: *const u16,
        options: *const CaptureOptions,
        handle: *mut u64,
    ) -> i32;
    fn im_session_destroy(handle: u64);
    fn im_session_attach_preview(handle: u64, hwnd: *mut c_void) -> i32;
    fn im_session_get_status(handle: u64, status: *mut RawStatus) -> i32;
    fn im_session_set_audio_enabled(handle: u64, enabled: i32) -> i32;
    fn im_session_set_window_rotation(handle: u64, hwnd: *mut c_void, rotation: i32) -> i32;
    fn im_wireless_receiver_start_ex(
        name: *const u16,
        path: *const u16,
        width: u32,
        height: u32,
        fps: u32,
    ) -> i32;
    fn im_wireless_receiver_stop();
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn text(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|v| *v == 0).unwrap_or(s.len())])
}
fn check(code: i32) -> Result<(), NativeError> {
    if code == 0 {
        return Ok(());
    }
    // SAFETY: im_last_error returns a NUL-terminated thread-local string valid
    // until the next native call on this thread. It is copied immediately.
    let message = unsafe {
        let ptr = im_last_error();
        if ptr.is_null() {
            String::new()
        } else {
            let mut len = 0;
            while len < 8192 && *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    };
    Err(NativeError { code, message })
}

/// One engine is owned by the device worker, which outlives all sessions.
pub struct Engine {
    _private: (),
}
impl Engine {
    pub fn new() -> Result<Arc<Self>, NativeError> {
        // SAFETY: Process-global native initialization is called by one worker.
        let version = unsafe { im_api_version() };
        if version != VERSION {
            return Err(NativeError {
                code: -1,
                message: "Media ABI version mismatch".into(),
            });
        }
        // SAFETY: No arguments; native implementation serializes initialization.
        if ENGINE_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(NativeError {
                code: -1,
                message: "A native engine already exists; share the existing Arc".into(),
            });
        }
        // SAFETY: The singleton guard grants exclusive native initialization.
        if let Err(error) = check(unsafe { im_initialize() }) {
            ENGINE_ACTIVE.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(Arc::new(Self { _private: () }))
    }
    pub fn devices(&self, metadata: bool) -> Result<Vec<Device>, NativeError> {
        let mut count = 0;
        // SAFETY: Null buffer is the documented count-query operation.
        check(unsafe { im_refresh_devices_ex(std::ptr::null_mut(), &mut count, metadata.into()) })?;
        if count > 64 {
            return Err(NativeError {
                code: -1,
                message: "Device count exceeds safety limit".into(),
            });
        }
        let mut devices = Vec::with_capacity(count as usize);
        for _ in 0..count {
            devices.push(DeviceInfo {
                size: size_of::<DeviceInfo>() as u32,
                version: VERSION,
                id: 0,
                port: 0,
                state: 0,
                usb: 0,
                pair: 0,
                lockdown: 0,
                udid: [0; 128],
                name: [0; 128],
                product: [0; 64],
                os: [0; 32],
                connection: [0; 32],
                status: [0; 192],
            });
        }
        // SAFETY: Vector contains count initialized ABI-compatible DeviceInfo entries.
        check(unsafe { im_refresh_devices_ex(devices.as_mut_ptr(), &mut count, metadata.into()) })?;
        Ok(devices
            .into_iter()
            .take(count as usize)
            .map(|d| Device {
                id: text(&d.udid),
                name: text(&d.name),
                model: text(&d.product),
                ios: text(&d.os),
                connection: text(&d.connection),
                status: text(&d.status),
                paired: d.pair != 0,
            })
            .collect())
    }
    pub fn connect(
        self: &Arc<Self>,
        device: &Device,
        options: CaptureOptions,
    ) -> Result<Session, NativeError> {
        let id = wide(&device.id);
        let mut handle = 0;
        // SAFETY: Strings are NUL-terminated; options/handle remain valid throughout this call.
        check(unsafe {
            if device.connection.to_ascii_lowercase().contains("airplay") {
                im_wireless_session_create(id.as_ptr(), &options, &mut handle)
            } else {
                im_session_create(id.as_ptr(), &options, &mut handle)
            }
        })?;
        Ok(Session {
            handle,
            _owner: Arc::clone(self),
            _timer: MediaTimer::new(),
        })
    }
    pub fn start_airplay(&self, name: &str, path: &str) -> Result<(), NativeError> {
        let name = wide(name);
        let path = wide(path);
        // SAFETY: Native code copies both NUL-terminated strings before returning.
        check(unsafe {
            im_wireless_receiver_start_ex(name.as_ptr(), path.as_ptr(), 1920, 1080, 60)
        })
    }
    pub fn stop_airplay(&self) {
        // SAFETY: Native stop is synchronized and idempotent.
        unsafe {
            im_wireless_receiver_stop();
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: Session-owned Arc references keep the engine alive until all sessions are destroyed.
        unsafe {
            im_shutdown();
        }
        ENGINE_ACTIVE.store(false, Ordering::Release);
    }
}

#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
    fn timeEndPeriod(period: u32) -> u32;
}
/// Keep native 1 ms pacing waits accurate only while media sessions are alive.
/// Windows 10 2004+ scopes this request to this process.
struct MediaTimer(bool);
impl MediaTimer {
    fn new() -> Self {
        // SAFETY: WinMM accepts a scalar millisecond period; success is balanced
        // exactly once in Drop and does not require thread affinity.
        Self(unsafe { timeBeginPeriod(1) == 0 })
    }
}
impl Drop for MediaTimer {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: This guard owns one successful 1 ms resolution request.
            unsafe {
                let _ = timeEndPeriod(1);
            }
        }
    }
}
pub struct Session {
    handle: u64,
    _owner: Arc<Engine>,
    _timer: MediaTimer,
}
impl Session {
    /// # Safety
    /// hwnd must be a valid live window, retained until this session is destroyed.
    pub unsafe fn attach(&self, hwnd: *mut c_void) -> Result<(), NativeError> {
        // SAFETY: The caller guarantees window lifetime; native code validates IsWindow.
        check(unsafe { im_session_attach_preview(self.handle, hwnd) })
    }
    pub fn status(&self) -> Result<Status, NativeError> {
        let mut raw = RawStatus {
            size: size_of::<RawStatus>() as u32,
            version: VERSION,
            state: 0,
            width: 0,
            height: 0,
            fps: 0.0,
            latency_ms: 0.0,
            frames: 0,
            audio_packets: 0,
            audio_rate: 0,
            audio_channels: 0,
            failure_kind: 0,
            failure_stage: 0,
            error: 0,
            message: [0; 192],
        };
        // SAFETY: raw is initialized with the exact C layout and validated version/size.
        check(unsafe { im_session_get_status(self.handle, &mut raw) })?;
        Ok(Status {
            state: raw.state,
            width: raw.width,
            height: raw.height,
            source_fps: (raw.fps >= 0.0 && raw.fps.is_finite()).then_some(raw.fps),
            last_decode_ms: raw.latency_ms,
            source_frames: raw.frames,
            error: raw.error,
            failure_kind: raw.failure_kind,
            failure_stage: raw.failure_stage,
            message: text(&raw.message),
        })
    }

    /// # Safety
    /// hwnd must remain a live preview owned by this session through the call.
    pub unsafe fn view_preferences(
        &self,
        hwnd: *mut c_void,
        one_to_one: bool,
        vsync: bool,
    ) -> Result<(), NativeError> {
        // SAFETY: The caller owns the preview lifetime; native access is synchronized.
        check(unsafe {
            im_session_set_view_preferences(self.handle, hwnd, one_to_one.into(), vsync.into())
        })
    }
    pub fn video_preferences(&self, width: u32, height: u32, fps: u32) -> Result<(), NativeError> {
        // SAFETY: Scalar preferences are validated natively against a live owned handle.
        check(unsafe { im_session_set_video_preferences(self.handle, width, height, fps) })
    }

    /// # Safety
    /// hwnd must be a live preview belonging to this session.
    pub unsafe fn render_submissions(&self, hwnd: *mut c_void) -> Result<u64, NativeError> {
        let mut count = 0;
        // SAFETY: Output is writable, session is owned, and caller retains its preview.
        check(unsafe { im_session_get_render_submissions(self.handle, hwnd, &mut count) })?;
        Ok(count)
    }

    pub fn decoder_mode(&self) -> &'static str {
        let mut output = RawOutputStatus {
            size: size_of::<RawOutputStatus>() as u32,
            version: VERSION,
            ..Default::default()
        };
        // SAFETY: Output has the matching C layout and the session owns a live handle.
        if unsafe {
            im_session_get_video_output_status(self.handle, std::ptr::null_mut(), &mut output)
        } != 0
        {
            return "unknown";
        }
        match output.runtime_mode {
            1 => "hardware",
            2 => "software",
            3 => "external",
            _ => "unknown",
        }
    }

    pub fn mute(&self, muted: bool) -> Result<(), NativeError> {
        // SAFETY: Session handle is owned and live, native code validates it.
        check(unsafe { im_session_set_audio_enabled(self.handle, (!muted).into()) })
    }
    /// # Safety
    /// hwnd must be the valid live window attached to this session.
    pub unsafe fn rotate(&self, hwnd: *mut c_void, quarter_turns: i32) -> Result<(), NativeError> {
        // SAFETY: Caller guarantees the HWND lifetime; handle is owned by this session.
        check(unsafe { im_session_set_window_rotation(self.handle, hwnd, quarter_turns) })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: This handle is destroyed exactly once; owner is still alive.
        unsafe {
            im_session_destroy(self.handle);
        }
    }
}

impl Engine {
    pub fn encoded(self: &Arc<Self>, options: CaptureOptions) -> Result<Session, NativeError> {
        let mut handle = 0;
        // SAFETY: Options has the current C layout; output handle is writable.
        check(unsafe { im_encoded_session_create(&options, &mut handle) })?;
        Ok(Session {
            handle,
            _owner: Arc::clone(self),
            _timer: MediaTimer::new(),
        })
    }
}
impl Session {
    #[allow(
        clippy::too_many_arguments,
        reason = "Mirrors the bounded encoded-video C ABI"
    )]
    pub fn submit_encoded(
        &self,
        avcc: &[u8],
        sps: &[u8],
        pps: &[u8],
        width: u32,
        height: u32,
        pts: i64,
        keyframe: bool,
        discontinuity: bool,
    ) -> Result<(), NativeError> {
        if avcc.len() > 8 * 1024 * 1024 || sps.len() > 65535 || pps.len() > 65535 {
            return Err(NativeError {
                code: -1,
                message: "Encoded packet exceeds safety limits".into(),
            });
        }
        // SAFETY: Borrowed slices remain live through the call. The native
        // receiver validates all lengths/ranges and copies into its bounded queue.
        check(unsafe {
            im_encoded_session_submit(
                self.handle,
                avcc.as_ptr(),
                avcc.len() as u32,
                sps.as_ptr(),
                sps.len() as u32,
                pps.as_ptr(),
                pps.len() as u32,
                width,
                height,
                pts,
                keyframe.into(),
                discontinuity.into(),
            )
        })
    }
    pub fn submit_pcm(&self, pcm: &[u8], rate: u32, channels: u32) -> Result<(), NativeError> {
        if pcm.len() > 65536 {
            return Err(NativeError {
                code: -1,
                message: "PCM packet too large".into(),
            });
        }
        // SAFETY: Native code validates PCM shape and copies the live borrowed slice.
        check(unsafe {
            im_encoded_session_audio(self.handle, pcm.as_ptr(), pcm.len() as u32, rate, channels)
        })
    }
    pub fn reset_encoded(&self) -> Result<(), NativeError> {
        // SAFETY: Session owns a live handle and the reset is synchronized natively.
        check(unsafe { im_encoded_session_reset(self.handle) })
    }
}
