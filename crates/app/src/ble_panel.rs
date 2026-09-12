//! BLE-only modeless diagnostics. No video viewport or media settings are changed.
use crate::control::Snapshot;
use imirror_input_core::{DEFAULT_POINTER_SPEED, MAX_POINTER_SPEED, MIN_POINTER_SPEED};
use std::ffi::c_void;
use windows::Win32::UI::Controls::{
    ICC_BAR_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, TBM_SETLINESIZE, TBM_SETPAGESIZE,
    TBM_SETPOS, TBM_SETRANGEMAX, TBM_SETRANGEMIN, TBS_AUTOTICKS,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::{EnableWindow, GetCapture},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};
pub const MOVE_RIGHT: usize = 420;
pub const MOVE_LEFT: usize = 421;
pub const LEFT_CLICK: usize = 422;
pub const TYPE_A: usize = 423;
pub const SELECT_TARGET: usize = 424;
const DETAILS: i32 = 425;
const TARGETS: i32 = 426;
pub const SET_SPEED: usize = 430;
const SPEED_LABEL: i32 = 431;
const SPEED_SLIDER: i32 = 432;
const SPEED_HINT: i32 = 433;
// CommCtrl.h defines TBM_GETPOS as WM_USER; this alias is omitted by windows-rs 0.61.
const TBM_GETPOS: u32 = WM_USER;
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
pub struct BlePanel {
    window: HWND,
    details: HWND,
    targets: HWND,
    speed: HWND,
    speed_label: HWND,
    buttons: [HWND; 4],
    clients: Vec<String>,
    last_text: String,
    language: Option<imirror_device::Language>,
}
impl BlePanel {
    pub fn create(owner: HWND) -> windows::core::Result<Self> {
        // SAFETY: Created and used only by the owning UI thread. Owner remains
        // alive throughout the modeless window's lifetime. Child text is copied.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let controls = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_BAR_CLASSES,
            };
            if !InitCommonControlsEx(&controls).as_bool() {
                return Err(windows::core::Error::from_win32());
            }
            let class = WNDCLASSW {
                hInstance: instance.into(),
                lpszClassName: w!("iMirrorBleDiagnostics"),
                lpfnWndProc: Some(panel_proc),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut c_void),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_win32());
            }
            let window = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("iMirrorBleDiagnostics"),
                w!("iMirror - Advanced Diagnostics / BLE control"),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                780,
                640,
                Some(owner),
                None,
                Some(instance.into()),
                Some(owner.0.cast_const()),
            )?;
            let result = (|| {
                crate::app_icon::apply(window)?;
                let speed_label = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("Mouse speed: loading..."),
                    WS_CHILD | WS_VISIBLE,
                    8,
                    8,
                    180,
                    26,
                    Some(window),
                    Some(HMENU(SPEED_LABEL as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                let speed = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("msctls_trackbar32"),
                    w!("Mouse speed"),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(TBS_AUTOTICKS),
                    190,
                    8,
                    550,
                    32,
                    Some(window),
                    Some(HMENU(SPEED_SLIDER as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                SendMessageW(
                    speed,
                    TBM_SETRANGEMIN,
                    Some(WPARAM(0)),
                    Some(LPARAM(MIN_POINTER_SPEED as isize)),
                );
                SendMessageW(
                    speed,
                    TBM_SETRANGEMAX,
                    Some(WPARAM(1)),
                    Some(LPARAM(MAX_POINTER_SPEED as isize)),
                );
                SendMessageW(speed, TBM_SETLINESIZE, None, Some(LPARAM(5)));
                SendMessageW(speed, TBM_SETPAGESIZE, None, Some(LPARAM(25)));
                SendMessageW(
                    speed,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(DEFAULT_POINTER_SPEED as isize)),
                );
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("Lower = slower. 100% = previous speed. Test buttons use fixed steps."),
                    WS_CHILD | WS_VISIBLE,
                    8,
                    44,
                    730,
                    24,
                    Some(window),
                    Some(HMENU(SPEED_HINT as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                let details = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    w!("Starting BLE diagnostics..."),
                    WS_CHILD
                        | WS_VISIBLE
                        | WS_BORDER
                        | WS_VSCROLL
                        | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32),
                    8,
                    8,
                    750,
                    450,
                    Some(window),
                    Some(HMENU(DETAILS as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                let targets = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("COMBOBOX"),
                    w!(""),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
                    8,
                    470,
                    750,
                    180,
                    Some(window),
                    Some(HMENU(TARGETS as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                let mut buttons = [HWND::default(); 4];
                for (index, (id, title)) in [
                    (MOVE_RIGHT, "Move Right"),
                    (MOVE_LEFT, "Move Left"),
                    (LEFT_CLICK, "Left Click"),
                    (TYPE_A, "Type A"),
                ]
                .iter()
                .enumerate()
                {
                    let title = wide(title);
                    buttons[index] = CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        w!("BUTTON"),
                        PCWSTR(title.as_ptr()),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        8 + index as i32 * 140,
                        510,
                        132,
                        30,
                        Some(window),
                        Some(HMENU(*id as *mut c_void)),
                        Some(instance.into()),
                        None,
                    )?;
                    let _ = EnableWindow(buttons[index], false);
                }
                Ok(Self {
                    window,
                    details,
                    targets,
                    speed,
                    speed_label,
                    buttons,
                    clients: Vec::new(),
                    last_text: String::new(),
                    language: None,
                })
            })();
            if result.is_err() {
                let _ = DestroyWindow(window);
            }
            if result.is_ok() {
                layout(window);
            }
            result
        }
    }
    pub fn show(&self) {
        // SAFETY: This modeless window belongs to the current UI thread.
        unsafe {
            let _ = ShowWindow(self.window, SW_SHOW);
            let _ = SetForegroundWindow(self.window);
        }
    }
    pub fn update(&mut self, input: &Snapshot) {
        let language = crate::i18n::language();
        if self.language != Some(language) {
            self.language = Some(language);
            // SAFETY: Update only this diagnostic window's title/buttons. Protocol
            // names, peer data and raw driver errors remain technical text.
            unsafe {
                let title = wide(&format!(
                    "iMirror — {}",
                    crate::i18n::tr("Advanced Diagnostics")
                ));
                let _ = SetWindowTextW(self.window, PCWSTR(title.as_ptr()));
                for (button, name) in
                    self.buttons
                        .iter()
                        .zip(["Move Right", "Move Left", "Left Click", "Type A"])
                {
                    let caption = wide(crate::i18n::tr(name));
                    let _ = SetWindowTextW(*button, PCWSTR(caption.as_ptr()));
                }
            }
        }
        let d = &input.ble;
        let percent =
            if (MIN_POINTER_SPEED..=MAX_POINTER_SPEED).contains(&input.pointer_speed_percent) {
                input.pointer_speed_percent
            } else {
                DEFAULT_POINTER_SPEED
            };
        let yes = |v: bool| if v { "YES" } else { "NO" };
        let adapter=d.adapter.as_ref().map(|a|format!(
            "Bluetooth adapter: {}\r\nPeripheral role supported: {} | LE supported: {}\r\nComputer name: {}\r\nAdapter ID: {}\r\nRadio state: {}",
            a.adapter_name,yes(a.peripheral),yes(a.low_energy),a.computer_name,a.adapter_id,a.radio_state))
            .unwrap_or_else(||"Bluetooth adapter: awaiting probe".into());
        let selected = d
            .selected_target
            .as_deref()
            .unwrap_or("NONE - select the iPhone below");
        let mouse = if d.mouse_subscribers.is_empty() {
            "(none)".into()
        } else {
            d.mouse_subscribers.join("\r\n")
        };
        let keyboard = if d.keyboard_subscribers.is_empty() {
            "(none)".into()
        } else {
            d.keyboard_subscribers.join("\r\n")
        };
        let observations = &d.enumeration;
        let pointer_clients = &d.mouse_subscribers;
        let events = observations
            .events
            .iter()
            .rev()
            .take(12)
            .rev()
            .map(|event| format!("{} {}", event.unix_ms, event.detail))
            .collect::<Vec<_>>()
            .join("\r\n");
        let peers = |values: &std::collections::BTreeMap<String, imirror_input_ble::Peer>| {
            values
                .values()
                .map(|p| format!("{} (paired at discovery={:?})", p.name, p.paired))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let stored = format!(
            "\r\nWindows stored subscriptions: mouse {}, keyboard {} (may remain cached after disconnect)",
            d.stored_mouse_subscriptions, d.stored_keyboard_subscriptions
        );
        let enumeration = format!(
            "{stored}\r\n\r\nBluetooth connected via LE: {} [{}]\r\nBluetooth connected via Classic: {} [{}]\r\nThese connections do not establish HID subscription.\r\nProtocol Mode: {} | Suspended: {}\r\nHID read counts: {:?}\r\nMouse subscription callbacks: {} | Keyboard subscription callbacks: {}\r\nGAP Appearance: {}\r\nCCCD: {}\r\n\r\nLive HID events (latest 12):\r\n{}",
            observations.connected_le.len(),
            peers(&observations.connected_le),
            observations.connected_classic.len(),
            peers(&observations.connected_classic),
            observations.protocol_mode,
            observations.suspended,
            observations.reads,
            observations.mouse_subscription_events,
            observations.keyboard_subscription_events,
            d.gap_appearance,
            d.cccd_support,
            events
        );
        let probe_note = "No movement test is sent automatically.";
        let settings_notice = input
            .pointer_settings_notice
            .as_deref()
            .unwrap_or("Speed preference is saved automatically.");
        let text = format!(
            "Mouse speed: {percent}% (relative mouse only)\r\n{settings_notice}\r\n\r\n{adapter}\r\n\r\nHID service 0x1812 created: {}\r\nAdvertising status: {}\r\nSTARTED observed: {} | startup timeout: {} | error: {:?}\r\n\r\nActive mouse report subscribers: {}\r\n{mouse}\r\nActive keyboard report subscribers: {}\r\n{keyboard}\r\n\r\nSelected target subscriber: {selected}\r\nMouse report ready: {} | keyboard report ready: {}\r\n\r\n{}\r\nError: {}\r\nLast diagnostic: {}\r\n\r\nThis milestone requires a real report subscription. {probe_note}\r\nLocal diagnostics: {}{enumeration}",
            yes(d.hid_service_created),
            d.advertising_status,
            yes(d.started_observed),
            yes(d.startup_timed_out),
            d.advertising_error,
            d.mouse_subscribers.len(),
            d.keyboard_subscribers.len(),
            yes(input.ready && d.mouse_ready()),
            yes(input.ready && d.keyboard_ready()),
            input.message,
            input.ble_error.as_deref().unwrap_or("none"),
            input.last_diagnostic.as_deref().unwrap_or("not sent"),
            crate::control::diagnostic_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unavailable".into())
        );
        let text = format!(
            "Control ready: {} | Transition queue: {}/{}\r\nDiagnostic file error: {}\r\nInput performance (software latency, not phone display latency):\r\n{:#?}\r\n\r\n{}",
            input.ready,
            input.transition_queue_depth,
            imirror_input_core::relative::MAX_TRANSITIONS,
            input.diagnostics_error.as_deref().unwrap_or("none"),
            input.performance,
            text
        );
        // SAFETY: HWNDs are retained by this panel; Win32 copies strings synchronously.
        unsafe {
            let _ = EnableWindow(self.speed, true);
            if GetCapture() != self.speed {
                SendMessageW(
                    self.speed,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(percent as isize)),
                );
                let caption = wide(&format!("{}: {percent}%", crate::i18n::tr("Mouse speed")));
                let _ = SetWindowTextW(self.speed_label, PCWSTR(caption.as_ptr()));
            }
            if text != self.last_text {
                let utf16 = wide(&text);
                let _ = SetWindowTextW(self.details, PCWSTR(utf16.as_ptr()));
                self.last_text = text;
            }
            if &self.clients != pointer_clients {
                SendMessageW(self.targets, CB_RESETCONTENT, None, None);
                for id in pointer_clients {
                    let id = wide(id);
                    SendMessageW(
                        self.targets,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(id.as_ptr() as isize)),
                    );
                }
                self.clients = pointer_clients.clone();
            }
            let selection = d
                .selected_target
                .as_ref()
                .and_then(|id| self.clients.iter().position(|x| x == id));
            SendMessageW(
                self.targets,
                CB_SETCURSEL,
                Some(WPARAM(selection.unwrap_or(usize::MAX))),
                None,
            );
            for (index, button) in self.buttons.iter().enumerate() {
                let _ = EnableWindow(
                    *button,
                    if index == 3 {
                        d.keyboard_ready()
                    } else {
                        d.mouse_ready()
                    },
                );
            }
        }
    }
}
impl Drop for BlePanel {
    fn drop(&mut self) {
        // SAFETY: Owner may already have destroyed its modeless child. Check validity first.
        unsafe {
            if IsWindow(Some(self.window)).as_bool() {
                let _ = DestroyWindow(self.window);
            }
        }
    }
}
fn layout(hwnd: HWND) {
    // SAFETY: Only handles of this live panel's own child controls are used.
    unsafe {
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return;
        }
        let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
        let px = |n: f64| (n * scale).round() as i32;
        let width = (rect.right - px(16.0)).max(1);
        let bottom = (rect.bottom - px(42.0)).max(px(80.0));
        if let Ok(control) = GetDlgItem(Some(hwnd), SPEED_LABEL) {
            let _ = MoveWindow(control, px(8.0), px(12.0), px(176.0), px(26.0), true);
        }
        if let Ok(control) = GetDlgItem(Some(hwnd), SPEED_SLIDER) {
            let _ = MoveWindow(
                control,
                px(188.0),
                px(8.0),
                (width - px(180.0)).max(1),
                px(32.0),
                true,
            );
        }
        if let Ok(control) = GetDlgItem(Some(hwnd), SPEED_HINT) {
            let _ = MoveWindow(control, px(8.0), px(44.0), width, px(26.0), true);
        }
        if let Ok(control) = GetDlgItem(Some(hwnd), DETAILS) {
            let _ = MoveWindow(
                control,
                px(8.0),
                px(76.0),
                width,
                (bottom - px(120.0)).max(1),
                true,
            );
        }
        if let Ok(control) = GetDlgItem(Some(hwnd), TARGETS) {
            let _ = MoveWindow(control, px(8.0), bottom - px(36.0), width, px(180.0), true);
        }
        for (index, id) in [MOVE_RIGHT, MOVE_LEFT, LEFT_CLICK, TYPE_A]
            .iter()
            .enumerate()
        {
            if let Ok(control) = GetDlgItem(Some(hwnd), *id as i32) {
                let cell = width / 4;
                let _ = MoveWindow(
                    control,
                    px(8.0) + index as i32 * cell,
                    bottom,
                    (cell - px(6.0)).max(1),
                    px(30.0),
                    true,
                );
            }
        }
    }
}
unsafe extern "system" fn panel_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: WM_NCCREATE provides CREATESTRUCTW. The saved owner HWND outlives
    // this window; commands contain integers only and are posted asynchronously.
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        match msg {
            WM_SIZE => {
                layout(hwnd);
                return LRESULT(0);
            }
            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            WM_HSCROLL => {
                if let Ok(slider) = GetDlgItem(Some(hwnd), SPEED_SLIDER)
                    && lparam.0 == slider.0 as isize
                {
                    let percent = SendMessageW(slider, TBM_GETPOS, None, None)
                        .0
                        .clamp(MIN_POINTER_SPEED as isize, MAX_POINTER_SPEED as isize);
                    if let Ok(label) = GetDlgItem(Some(hwnd), SPEED_LABEL) {
                        let caption =
                            wide(&format!("{}: {percent}%", crate::i18n::tr("Mouse speed")));
                        let _ = SetWindowTextW(label, PCWSTR(caption.as_ptr()));
                    }
                    let owner = HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut c_void);
                    let _ =
                        PostMessageW(Some(owner), WM_COMMAND, WPARAM(SET_SPEED), LPARAM(percent));
                }
                return LRESULT(0);
            }
            WM_COMMAND => {
                let id = wparam.0 & 0xffff;
                let owner = HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut c_void);
                if (MOVE_RIGHT..=TYPE_A).contains(&id) {
                    let _ = PostMessageW(Some(owner), WM_COMMAND, WPARAM(id), LPARAM(0));
                } else if id == TARGETS as usize
                    && (wparam.0 >> 16) as u32 == CBN_SELCHANGE
                    && let Ok(targets) = GetDlgItem(Some(hwnd), TARGETS)
                {
                    let index = SendMessageW(targets, CB_GETCURSEL, None, None);
                    if index.0 >= 0 {
                        let _ = PostMessageW(
                            Some(owner),
                            WM_COMMAND,
                            WPARAM(SELECT_TARGET),
                            LPARAM(index.0),
                        );
                    }
                }
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
