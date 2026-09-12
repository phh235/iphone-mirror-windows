//! Native UI capture ownership and input observations. No phone input is sent here.
use std::cell::{Cell, RefCell};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetCapture, GetFocus, IsWindowEnabled, ReleaseCapture},
        WindowsAndMessaging::*,
    },
};

const IDS: [i32; 6] = [101, 105, 108, 111, 112, 117];
const NAMES: [&str; 6] = [
    "Connect/Disconnect",
    "Rotate",
    "Control",
    "Home",
    "Fullscreen",
    "Settings",
];
pub const COMMAND_EXECUTED: u32 = WM_APP + 93;
pub const PREVIEW_FOCUS_LOST: u32 = WM_APP + 94;
#[derive(Clone, Copy, Default)]
struct Counts {
    down: u64,
    up: u64,
    clicked: u64,
    executed: u64,
    capture_lost: u64,
}
thread_local! {
    static KEYBOARD:Cell<bool>=const{Cell::new(false)};
    static COUNTS:RefCell<[Counts;6]>=const{RefCell::new([Counts{down:0,up:0,clicked:0,executed:0,capture_lost:0};6])};
}
pub fn keyboard_focus_visible() -> bool {
    KEYBOARD.get()
}
pub fn modality(keyboard: bool) {
    if KEYBOARD.replace(keyboard) != keyboard {
        // SAFETY: Invalidate only this thread's focused native control; no input is synthesized.
        unsafe {
            let focus = GetFocus();
            if !focus.0.is_null() {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(focus), None, false);
            }
        }
    }
}
pub fn observe_message(message: &MSG) {
    match message.message {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_NCLBUTTONDOWN => modality(false),
        WM_KEYDOWN if matches!(message.wParam.0, 0x09 | 0x0d | 0x20 | 0x25..=0x28) => {
            modality(true)
        }
        _ => {}
    }
}
/// A delayed preview focus notification must never release a native button's capture.
pub fn release_capture_if_owned(preview: HWND) -> bool {
    // SAFETY: GetCapture refers to this UI thread. Only the named preview's capture is released.
    unsafe {
        if !preview.0.is_null() && GetCapture() == preview {
            return ReleaseCapture().is_ok();
        }
    }
    false
}
pub fn default_button(window: HWND) -> LRESULT {
    // SAFETY: Resolve only a focused, enabled BUTTON below this owned top-level window.
    unsafe {
        let focus = GetFocus();
        if focus.0.is_null()
            || !IsChild(window, focus).as_bool()
            || !IsWindowEnabled(focus).as_bool()
        {
            return LRESULT(0);
        }
        let mut class = [0u16; 16];
        let n = GetClassNameW(focus, &mut class).max(0) as usize;
        let style = GetWindowLongW(focus, GWL_STYLE) as u32 & 0xf;
        if String::from_utf16_lossy(&class[..n]).eq_ignore_ascii_case("Button")
            && matches!(style, 0 | 1 | 11)
        {
            let id = GetDlgCtrlID(focus);
            if id > 0 {
                return LRESULT(((0x534b_u32 << 16) | id as u32) as isize);
            }
        }
    }
    LRESULT(0)
}
pub fn native_click(window: HWND, wparam: WPARAM, lparam: LPARAM) -> bool {
    if wparam.0 >> 16 != BN_CLICKED as usize || lparam.0 == 0 {
        return false;
    }
    // SAFETY: Validate the notification's control HWND and ID against the parent.
    unsafe {
        GetDlgItem(Some(window), (wparam.0 & 0xffff) as i32)
            .is_ok_and(|child| child.0 as isize == lparam.0)
    }
}
pub fn record(window: HWND, message: u32, detail: usize) {
    // SAFETY: Callers supply a live UI HWND; no device or credential data is inspected.
    let id = unsafe { GetDlgCtrlID(window) };
    if let Some(index) = IDS.iter().position(|&v| v == id) {
        COUNTS.with_borrow_mut(|counts| {
            let c = &mut counts[index];
            match message {
                WM_LBUTTONDOWN => c.down += 1,
                WM_LBUTTONUP => c.up += 1,
                WM_COMMAND => c.clicked += 1,
                COMMAND_EXECUTED => c.executed += 1,
                WM_CAPTURECHANGED => c.capture_lost += 1,
                _ => {}
            }
        });
    }
    #[cfg(any(debug_assertions, feature = "ui-input-trace"))]
    trace::record(window, id, message, detail);
    #[cfg(not(any(debug_assertions, feature = "ui-input-trace")))]
    let _ = detail;
}
pub fn snapshot() -> serde_json::Value {
    COUNTS.with_borrow(|counts|serde_json::json!({
        "note":"Native UI observations; keyboard activations have no mouse-down/up. Counters are not physical acceptance.",
        "buttons":counts.iter().enumerate().map(|(i,c)|serde_json::json!({"id":IDS[i],"name":NAMES[i],"mouse_down":c.down,"mouse_up":c.up,"bn_clicked":c.clicked,"command_executed":c.executed,"capture_changed":c.capture_lost})).collect::<Vec<_>>()
    }))
}

#[cfg(any(debug_assertions, feature = "ui-input-trace"))]
mod trace {
    use super::*;
    use std::{
        io::Write,
        sync::{
            Arc, OnceLock,
            atomic::{AtomicBool, AtomicU64, Ordering},
            mpsc::{SyncSender, sync_channel},
        },
        thread::{self, JoinHandle},
        time::Duration,
    };
    #[derive(Clone, Copy)]
    struct Event {
        at: u64,
        hwnd: usize,
        id: i32,
        message: u32,
        detail: usize,
        capture: usize,
        focus: usize,
    }
    static SENDER: OnceLock<SyncSender<Event>> = OnceLock::new();
    static DROPPED: AtomicU64 = AtomicU64::new(0);
    pub struct Guard {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }
    impl Guard {
        pub fn start() -> std::io::Result<Self> {
            let stop = Arc::new(AtomicBool::new(false));
            let args: Vec<_> = std::env::args_os().collect();
            let Some(index) = args.iter().position(|a| a == "--ui-button-trace") else {
                return Ok(Self { stop, thread: None });
            };
            let path = std::path::PathBuf::from(
                args.get(index + 1)
                    .ok_or_else(|| std::io::Error::other("--ui-button-trace requires a path"))?,
            );
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::File::create(path)?;
            let (sender, receiver) = sync_channel::<Event>(512);
            SENDER
                .set(sender)
                .map_err(|_| std::io::Error::other("UI trace already started"))?;
            let stopping = stop.clone();
            let thread=thread::Builder::new().name("imirror-ui-trace".into()).spawn(move||{
                let mut bytes=0usize;
                while !stopping.load(Ordering::Acquire) {
                    if let Ok(e)=receiver.recv_timeout(Duration::from_millis(20)) {
                        let name=match e.message {WM_MOUSEACTIVATE=>"WM_MOUSEACTIVATE",WM_LBUTTONDOWN=>"WM_LBUTTONDOWN",WM_LBUTTONUP=>"WM_LBUTTONUP",BM_SETSTATE=>"BM_SETSTATE",WM_COMMAND=>"WM_COMMAND/BN_CLICKED",COMMAND_EXECUTED=>"COMMAND_EXECUTED",PREVIEW_FOCUS_LOST=>"PREVIEW_FOCUS_LOST",WM_CAPTURECHANGED=>"WM_CAPTURECHANGED",WM_SETFOCUS=>"WM_SETFOCUS",WM_KILLFOCUS=>"WM_KILLFOCUS",_=>"OTHER"};
                        if let Ok(mut line)=serde_json::to_vec(&serde_json::json!({"monotonic_ns":e.at,"hwnd":e.hwnd,"control_id":e.id,"event":name,"detail":e.detail,"capture":e.capture,"focus":e.focus})) {
                            line.push(b'\n');
                            if bytes+line.len()<=4*1024*1024 {bytes+=line.len();let _=file.write_all(&line);let _=file.flush();}else{DROPPED.fetch_add(1,Ordering::Relaxed);}
                        }
                    }
                }
                let _=writeln!(file,"{{\"dropped_trace_events\":{}}}",DROPPED.load(Ordering::Relaxed));
            })?;
            Ok(Self {
                stop,
                thread: Some(thread),
            })
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
    pub fn record(window: HWND, id: i32, message: u32, detail: usize) {
        if let Some(sender) = SENDER.get() {
            // SAFETY: Read-only observations on the owning native UI thread.
            let event = unsafe {
                Event {
                    at: imirror_input_core::metrics::now_ns(),
                    hwnd: window.0 as usize,
                    id,
                    message,
                    detail,
                    capture: GetCapture().0 as usize,
                    focus: GetFocus().0 as usize,
                }
            };
            if sender.try_send(event).is_err() {
                DROPPED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
#[cfg(any(debug_assertions, feature = "ui-input-trace"))]
pub use trace::Guard as TraceGuard;

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{Win32::UI::Input::KeyboardAndMouse::SetCapture, core::w};
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            // SAFETY: Test owns this hidden message-only window and its children.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    #[test]
    fn delayed_preview_cleanup_preserves_native_button_capture() -> windows::core::Result<()> {
        // No clicks/commands are injected. Exercise actual Win32 capture ownership
        // using private hidden test windows, not the user's app or phone.
        // SAFETY: Each HWND is created/destroyed on this test thread.
        unsafe {
            let root = Window(CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE::default(),
                0,
                0,
                1,
                1,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )?);
            let preview = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD,
                0,
                0,
                1,
                1,
                Some(root.0),
                None,
                None,
                None,
            )?;
            let button = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Test"),
                WS_CHILD | WINDOW_STYLE(BS_OWNERDRAW as u32),
                0,
                0,
                1,
                1,
                Some(root.0),
                Some(HMENU(101usize as *mut _)),
                None,
                None,
            )?;
            SetCapture(button);
            ReleaseCapture()?;
            assert!(
                GetCapture().0.is_null(),
                "Old unconditional release cancels the button's capture"
            );
            for _ in 0..100 {
                SetCapture(button);
                assert!(!release_capture_if_owned(preview));
                assert_eq!(GetCapture(), button);
                SetCapture(preview);
                assert!(release_capture_if_owned(preview));
                assert!(GetCapture().0.is_null());
            }
            assert!(native_click(root.0, WPARAM(101), LPARAM(button.0 as isize)));
            assert!(!native_click(
                root.0,
                WPARAM(101 | (BN_SETFOCUS as usize) << 16),
                LPARAM(button.0 as isize)
            ));
        }
        Ok(())
    }
}
