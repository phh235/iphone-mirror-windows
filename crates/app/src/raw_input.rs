//! Dedicated Win32 Raw Input receiver. Emergency release never waits for BLE or UI work.
use imirror_input_core::{metrics::now_ns, relative::Mailbox};
use std::{
    ffi::c_void,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL},
        },
        UI::{
            Input::{KeyboardAndMouse::*, *},
            WindowsAndMessaging::*,
        },
    },
    core::w,
};
const CAPTURE: u32 = WM_APP + 41;
const RELEASE: u32 = WM_APP + 42;
const STOP: u32 = WM_APP + 43;
const HOTKEY: i32 = 0x4951;
#[derive(Clone, Copy)]
struct CaptureRequest {
    owner: usize,
    bounds: RECT,
    received: u64,
}
struct Shared {
    hwnd: AtomicUsize,
    request: Mutex<Option<CaptureRequest>>,
    hotkey: AtomicBool,
    mailbox: Arc<Mailbox>,
}
struct Context {
    shared: Arc<Shared>,
    owner: usize,
    buttons: u8,
    modifiers: u8,
    keys: [u8; 6],
    capturing: bool,
    ignore_activation_left: bool,
}
pub struct RawInput {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}
impl RawInput {
    pub fn start(mailbox: Arc<Mailbox>) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            hwnd: AtomicUsize::new(0),
            request: Mutex::new(None),
            hotkey: AtomicBool::new(false),
            mailbox,
        });
        let state = shared.clone();
        let (started, ready) = std::sync::mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("imirror-raw-input".into())
            .spawn(move || {
                let result = (|| -> windows::core::Result<()> {
                    let mut context = Box::new(Context {
                        shared: state.clone(),
                        owner: 0,
                        buttons: 0,
                        modifiers: 0,
                        keys: [0; 6],
                        capturing: false,
                        ignore_activation_left: false,
                    });
                    // SAFETY: Class and message window belong to this thread; context outlives window dispatch.
                    unsafe {
                        let module = GetModuleHandleW(None)?;
                        let class = WNDCLASSW {
                            lpfnWndProc: Some(procedure),
                            hInstance: module.into(),
                            lpszClassName: w!("iMirrorRawInput"),
                            ..Default::default()
                        };
                        if RegisterClassW(&class) == 0 {
                            return Err(windows::core::Error::from_win32());
                        }
                        let window = CreateWindowExW(
                            WINDOW_EX_STYLE::default(),
                            w!("iMirrorRawInput"),
                            w!(""),
                            WINDOW_STYLE::default(),
                            0,
                            0,
                            0,
                            0,
                            Some(HWND_MESSAGE),
                            None,
                            Some(module.into()),
                            Some((&mut *context as *mut Context).cast::<c_void>()),
                        )?;
                        let setup = (|| -> windows::core::Result<()> {
                            register(window, false)?;
                            RegisterHotKey(
                                Some(window),
                                HOTKEY,
                                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                                0x51,
                            )?;
                            state.hotkey.store(true, Ordering::Release);
                            state.hwnd.store(window.0 as usize, Ordering::Release);
                            let _ =
                                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
                            SetTimer(Some(window), 1, 50, None);
                            let _ = started.send(Ok(()));
                            let mut message = MSG::default();
                            loop {
                                let result = GetMessageW(&mut message, None, 0, 0);
                                if result.0 <= 0 {
                                    break;
                                }
                                let _ = TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                            Ok(())
                        })();
                        context.release(window);
                        let _ = UnregisterHotKey(Some(window), HOTKEY);
                        state.hotkey.store(false, Ordering::Release);
                        state.hwnd.store(0, Ordering::Release);
                        let _ = DestroyWindow(window);
                        setup
                    }
                })();
                if let Err(error) = result {
                    let _ =
                        started.send(Err(format!("Raw Input / Ctrl+Alt+Q unavailable: {error}")));
                }
            })
            .map_err(|e| e.to_string())?;
        match ready.recv() {
            Ok(Ok(())) => Ok(Self {
                shared,
                thread: Some(thread),
            }),
            other => {
                let _ = thread.join();
                Err(format!("Raw Input startup failed: {other:?}"))
            }
        }
    }
    pub fn hotkey_ready(&self) -> bool {
        self.shared.hotkey.load(Ordering::Acquire)
    }
    pub fn invalidation_handler(&self) -> Arc<dyn Fn() + Send + Sync> {
        let shared = self.shared.clone();
        Arc::new(move || {
            shared.mailbox.invalidate();
            let window = HWND(shared.hwnd.load(Ordering::Acquire) as *mut c_void);
            if !window.0.is_null() {
                // SAFETY: Scalar release message to this receiver's owned window;
                // the callback never waits for BLE or the UI thread.
                unsafe {
                    let _ = PostMessageW(Some(window), RELEASE, WPARAM(0), LPARAM(0));
                }
            }
        })
    }
    pub fn capture(&self, owner: HWND, bounds: RECT) {
        *self
            .shared
            .request
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(CaptureRequest {
            owner: owner.0 as usize,
            bounds,
            received: now_ns(),
        });
        self.post(CAPTURE);
    }
    pub fn release(&self) {
        self.shared.mailbox.release();
        self.post(RELEASE);
    }
    fn post(&self, message: u32) {
        let window = HWND(self.shared.hwnd.load(Ordering::Acquire) as *mut c_void);
        if !window.0.is_null() {
            // SAFETY: Receiver owns HWND; queued scalar messages carry no borrowed data.
            unsafe {
                let _ = PostMessageW(Some(window), message, WPARAM(0), LPARAM(0));
            }
        }
    }
    pub fn request_stop(&self) {
        self.release();
        self.post(STOP);
    }
    pub fn is_stopped(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub fn stop(&mut self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for RawInput {
    fn drop(&mut self) {
        self.stop();
    }
}
fn register(window: HWND, captured: bool) -> windows::core::Result<()> {
    let flags = if captured {
        RIDEV_INPUTSINK | RIDEV_NOLEGACY
    } else {
        RIDEV_INPUTSINK
    };
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 2,
            dwFlags: flags,
            hwndTarget: window,
        },
        RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 6,
            dwFlags: flags,
            hwndTarget: window,
        },
    ];
    // SAFETY: Fixed initialized descriptors register mouse and keyboard only for this owned message window.
    unsafe { RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32) }
}
impl Context {
    fn release(&mut self, window: HWND) {
        if self.capturing {
            // SAFETY: Only remove the confinement this context installed; no BLE/UI wait precedes it.
            unsafe {
                let _ = ClipCursor(None);
            }
            self.shared.mailbox.release();
            self.capturing = false;
            let _ = register(window, false);
        }
        self.keys = [0; 6];
        self.modifiers = 0;
        self.buttons = 0;
        self.ignore_activation_left = false;
    }
    fn capture(&mut self, window: HWND) {
        let request = self
            .shared
            .request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let Some(request) = request else {
            return;
        };
        // SAFETY: Owner identity and rectangle were supplied by our live UI; reject lost foreground ownership.
        if unsafe { GetForegroundWindow().0 as usize } != request.owner {
            return;
        }
        if !self.shared.mailbox.capture(0, request.received) {
            return;
        }
        self.owner = request.owner;
        // Taking capture is not a remote button press. In particular, a late UI
        // capture request must not recreate a DOWN whose physical UP already passed.
        self.buttons = 0;
        // SAFETY: One-time physical button-state query; mouse movement still comes exclusively from WM_INPUT.
        self.ignore_activation_left = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0;
        self.modifiers = 0;
        self.keys = [0; 6];
        self.capturing = true;
        // SAFETY: Rectangle is in screen pixels and valid only during capture; every release path clears it.
        if register(window, true).is_err() || unsafe { ClipCursor(Some(&request.bounds)) }.is_err()
        {
            self.release(window);
        }
    }
    fn input(&mut self, window: HWND, lparam: LPARAM) {
        let received = now_ns();
        let mut raw = RAWINPUT::default();
        let mut size = std::mem::size_of::<RAWINPUT>() as u32;
        // SAFETY: GetRawInputData receives a correctly aligned fixed buffer of the declared size.
        let result = unsafe {
            GetRawInputData(
                HRAWINPUT(lparam.0 as *mut c_void),
                RID_INPUT,
                Some((&mut raw as *mut RAWINPUT).cast()),
                &mut size,
                std::mem::size_of::<RAWINPUTHEADER>() as u32,
            )
        };
        if result == u32::MAX
            || result < std::mem::size_of::<RAWINPUTHEADER>() as u32
            || size > std::mem::size_of::<RAWINPUT>() as u32
        {
            return;
        }
        if !self.capturing {
            return;
        }
        // SAFETY: Foreground getter has no borrowed memory and the owner handle is only compared.
        if !self.shared.mailbox.captured.load(Ordering::Acquire)
            || !self.shared.mailbox.ready.load(Ordering::Acquire)
            || unsafe { GetForegroundWindow().0 as usize } != self.owner
        {
            self.release(window);
            return;
        }
        if raw.header.dwType == RIM_TYPEMOUSE.0 {
            // SAFETY: Header identifies a RAWMOUSE payload and the complete fixed buffer was validated.
            let mouse = unsafe { raw.data.mouse };
            // Absolute devices (e.g. some remote-desktop pointers) are not relative physical mice.
            if mouse.usFlags.0 & MOUSE_MOVE_ABSOLUTE.0 != 0 {
                self.shared
                    .mailbox
                    .metrics
                    .rejected
                    .fetch_add(1, Ordering::Relaxed);
                return;
            }
            // SAFETY: Both union fields describe the same initialized RAWMOUSE button word.
            let mut flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags };
            if self.ignore_activation_left {
                // Suppress the click that activated capture. The state fallback covers
                // an UP that arrived during Raw Input registration, without synthesizing input.
                // SAFETY: Physical button state only; no cursor polling or injection.
                if u32::from(flags) & RI_MOUSE_LEFT_BUTTON_UP != 0
                    || unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } >= 0
                {
                    self.ignore_activation_left = false;
                }
                flags &= !((RI_MOUSE_LEFT_BUTTON_DOWN | RI_MOUSE_LEFT_BUTTON_UP) as u16);
            }
            let extracted = now_ns();
            self.shared.mailbox.metrics.input_times.record(received);
            self.shared
                .mailbox
                .metrics
                .extract
                .record(extracted.saturating_sub(received));
            self.shared.mailbox.mouse(
                mouse.lLastX,
                mouse.lLastY,
                self.buttons,
                0,
                received,
                extracted,
            );
            for (down, up, mask) in [
                (RI_MOUSE_LEFT_BUTTON_DOWN, RI_MOUSE_LEFT_BUTTON_UP, 1),
                (RI_MOUSE_RIGHT_BUTTON_DOWN, RI_MOUSE_RIGHT_BUTTON_UP, 2),
                (RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_MIDDLE_BUTTON_UP, 4),
            ] {
                if u32::from(flags) & down != 0 {
                    self.buttons |= mask;
                    self.shared
                        .mailbox
                        .mouse(0, 0, self.buttons, 0, received, extracted);
                }
                if u32::from(flags) & up != 0 {
                    self.buttons &= !mask;
                    self.shared
                        .mailbox
                        .mouse(0, 0, self.buttons, 0, received, extracted);
                }
            }
            let wheel = if u32::from(flags) & RI_MOUSE_WHEEL != 0 {
                // SAFETY: Wheel data is the signed high word of the validated mouse payload.
                unsafe { mouse.Anonymous.Anonymous.usButtonData as i16 as i32 }
            } else {
                0
            };
            if wheel != 0 {
                self.shared
                    .mailbox
                    .mouse(0, 0, self.buttons, wheel, received, extracted);
            }
        } else if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
            // SAFETY: Header identifies a complete RAWKEYBOARD payload.
            let key = unsafe { raw.data.keyboard };
            let down = key.Flags & RI_KEY_BREAK as u16 == 0;
            if let Some(mask) = modifier(key.VKey, key.MakeCode, key.Flags) {
                if down {
                    self.modifiers |= mask;
                } else {
                    self.modifiers &= !mask;
                }
            }
            if down
                && (key.VKey == 0x1b
                    || (key.VKey == 0x51
                        && self.modifiers & 0x11 != 0
                        && self.modifiers & 0x44 != 0))
            {
                self.release(window);
                return;
            }
            if let Some(usage) = key_usage(key.VKey) {
                if down
                    && key.VKey == 0x46
                    && self.modifiers & 0x11 != 0
                    && self.modifiers & 0x22 != 0
                {
                    self.release(window);
                    // SAFETY: Scalar fullscreen command to the capture owner; no foreign window input is injected.
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(self.owner as *mut c_void)),
                            WM_COMMAND,
                            WPARAM(112),
                            LPARAM(0),
                        );
                    }
                    return;
                }
                if down
                    && !self.keys.contains(&usage)
                    && let Some(slot) = self.keys.iter_mut().find(|v| **v == 0)
                {
                    *slot = usage;
                }
                if !down {
                    for slot in &mut self.keys {
                        if *slot == usage {
                            *slot = 0;
                        }
                    }
                }
            }
            self.shared.mailbox.key(self.modifiers, self.keys, received);
        }
        if !self.shared.mailbox.captured.load(Ordering::Acquire) {
            self.release(window);
        }
    }
}
unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Context is installed from our CreateWindowEx argument and lives until DestroyWindow returns.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Context;
        if !ptr.is_null() {
            let context = &mut *ptr;
            match message {
                WM_INPUT => {
                    context.input(window, lparam);
                    return DefWindowProcW(window, message, wparam, lparam);
                }
                WM_HOTKEY if wparam.0 == HOTKEY as usize => {
                    context.release(window);
                    return LRESULT(0);
                }
                CAPTURE => {
                    context.capture(window);
                    return LRESULT(0);
                }
                RELEASE => {
                    context.release(window);
                    return LRESULT(0);
                }
                STOP => {
                    context.release(window);
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                WM_TIMER => {
                    if context.capturing
                        && (!context.shared.mailbox.captured.load(Ordering::Acquire)
                            || !context.shared.mailbox.ready.load(Ordering::Acquire)
                            || GetForegroundWindow().0 as usize != context.owner)
                    {
                        context.release(window);
                    }
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
fn modifier(vk: u16, scan: u16, flags: u16) -> Option<u8> {
    match vk {
        0x10 => Some(if scan == 0x36 { 0x20 } else { 0x02 }),
        0x11 => Some(if flags & RI_KEY_E0 as u16 != 0 {
            0x10
        } else {
            0x01
        }),
        0x12 => Some(if flags & RI_KEY_E0 as u16 != 0 {
            0x40
        } else {
            0x04
        }),
        0x5b => Some(0x08),
        0x5c => Some(0x80),
        _ => None,
    }
}
pub fn key_usage(vk: u16) -> Option<u8> {
    match vk {
        0x41..=0x5a => Some((vk - 0x41 + 4) as u8),
        0x31..=0x39 => Some((vk - 0x31 + 30) as u8),
        0x30 => Some(39),
        0x0d => Some(40),
        0x08 => Some(42),
        0x09 => Some(43),
        0x20 => Some(44),
        0x25 => Some(80),
        0x26 => Some(82),
        0x27 => Some(79),
        0x28 => Some(81),
        0x2e => Some(76),
        0x70..=0x7b => Some((vk - 0x70 + 58) as u8),
        0xba => Some(51),
        0xbb => Some(46),
        0xbc => Some(54),
        0xbd => Some(45),
        0xbe => Some(55),
        0xbf => Some(56),
        0xc0 => Some(53),
        0xdb => Some(47),
        0xdc => Some(49),
        0xdd => Some(48),
        0xde => Some(52),
        _ => None,
    }
}
