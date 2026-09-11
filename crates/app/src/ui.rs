use crate::input::{Command as InputCommand, InputWorker, Snapshot as InputSnapshot};
use crate::worker::{Command, Snapshot, Worker};
use imirror_coordinate_map::{Mapper, Point, Rect, Rotation, ScaleMode, Size};
use imirror_input_core::{Button, Gesture, Input};
use std::time::Instant;
use std::{cell::RefCell, ffi::c_void};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BLACK_BRUSH, COLOR_WINDOW, GetStockObject, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
                SetProcessDpiAwarenessContext,
            },
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

const CONNECT: usize = 101;
const DISCONNECT: usize = 102;
const REFRESH: usize = 103;
const ROTATE: usize = 105;
const DEVICE: usize = 106;
const INFO: usize = 107;
const BLE: usize = 108;
const WDA: usize = 109;
const DISABLE: usize = 110;
const HOME: usize = 111;
const FULL: usize = 112;
const AIRPLAY: usize = 115;
const FIT: usize = 116;
const SETTINGS: usize = 117;
const DIRECT_TOUCH: usize = 118;
const SETTINGS_FIRST: usize = 130;
const SETTINGS_APPLY: usize = 136;
const BLE_DEVICE: usize = 113;
const INPUT_INFO: usize = 114;

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn label(hwnd: HWND, value: &str) {
    let value = wide(value);
    // SAFETY: HWND is owned by this UI thread; SetWindowText copies the string.
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(value.as_ptr()));
    }
}
struct Ui {
    worker: Worker,
    snapshot: Snapshot,
    preview: HWND,
    devices: HWND,
    info: HWND,
    controls: Vec<HWND>,
    rotation: i32,
    input: InputWorker,
    input_snapshot: InputSnapshot,
    input_info: HWND,
    ble_devices: HWND,
    ble_panel: Option<crate::ble_panel::BlePanel>,
    gesture: Gesture,
    captured: bool,
    last_mouse: Option<Point>,
    keys: Vec<u8>,
    surrogate: Option<u16>,
    fullscreen: Option<RECT>,
    started: Instant,
    smoke_test: bool,
    config: imirror_device::Config,
    settings_visible: bool,
    settings_controls: Vec<HWND>,
}
impl Ui {
    fn command(&self, command: Command) {
        if self.worker.commands.try_send(command).is_err() {
            label(
                self.info,
                "A device operation is still running. Please wait.",
            );
        }
    }
}
pub fn run(smoke_test: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::settings::load()?;
    let state = Box::new(RefCell::new(Ui {
        worker: Worker::start(config.clone())?,
        snapshot: Snapshot::default(),
        preview: HWND::default(),
        devices: HWND::default(),
        info: HWND::default(),
        controls: Vec::new(),
        rotation: 0,
        input: InputWorker::start()?,
        input_snapshot: InputSnapshot::default(),
        input_info: HWND::default(),
        ble_devices: HWND::default(),
        ble_panel: None,
        gesture: Gesture::default(),
        captured: false,
        last_mouse: None,
        keys: Vec::new(),
        surrogate: None,
        fullscreen: None,
        started: Instant::now(),
        smoke_test,
        config,
        settings_visible: false,
        settings_controls: Vec::new(),
    }));
    // SAFETY: All window registration and lifetime operations run on this UI thread.
    // state remains allocated until every owned HWND has been destroyed.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: w!("iMirrorWindow"),
            lpfnWndProc: Some(window_proc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut c_void),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(windows::core::Error::from_win32().into());
        }
        let preview_class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: w!("iMirrorVideo"),
            lpfnWndProc: Some(preview_proc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ..Default::default()
        };
        if RegisterClassW(&preview_class) == 0 {
            return Err(windows::core::Error::from_win32().into());
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("iMirrorWindow"),
            w!("iMirror"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            900,
            None,
            None,
            Some(instance.into()),
            Some(&*state as *const RefCell<Ui> as *const c_void),
        )?;
        let _lifetime = WindowLifetime {
            hwnd,
            state: &state,
        };
        let devices = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            8,
            8,
            360,
            250,
            Some(hwnd),
            Some(HMENU(DEVICE as *mut c_void)),
            Some(instance.into()),
            None,
        )?;
        state.borrow_mut().devices = devices;
        for (id, title) in [
            (CONNECT, "Connect"),
            (DISCONNECT, "Disconnect"),
            (REFRESH, "Scan"),
            (ROTATE, "Rotate"),
            (BLE, "BLE mouse"),
            (DIRECT_TOUCH, "Direct touch"),
            (WDA, "WDA control"),
            (DISABLE, "Release"),
            (HOME, "Home"),
            (FULL, "Fullscreen"),
            (AIRPLAY, "AirPlay"),
            (FIT, "1:1"),
            (SETTINGS, "Settings"),
        ] {
            let title = wide(title);
            let control = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                PCWSTR(title.as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                0,
                0,
                80,
                28,
                Some(hwnd),
                Some(HMENU(id as *mut c_void)),
                Some(instance.into()),
                None,
            )?;
            state.borrow_mut().controls.push(control);
        }
        let info = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Connect your iPhone by USB, unlock it, and tap Trust This Computer."),
            WS_CHILD | WS_VISIBLE,
            8,
            72,
            580,
            55,
            Some(hwnd),
            Some(HMENU(INFO as *mut c_void)),
            Some(instance.into()),
            None,
        )?;
        state.borrow_mut().info = info;
        let preview = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("iMirrorVideo"),
            w!(""),
            WS_CHILD | WS_VISIBLE,
            0,
            130,
            600,
            730,
            Some(hwnd),
            None,
            Some(instance.into()),
            None,
        )?;
        state.borrow_mut().preview = preview;
        let ble_devices = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            0,
            0,
            350,
            180,
            Some(hwnd),
            Some(HMENU(BLE_DEVICE as *mut c_void)),
            Some(instance.into()),
            None,
        )?;
        state.borrow_mut().ble_devices = ble_devices;
        let input_info = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Control is off"),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            580,
            55,
            Some(hwnd),
            Some(HMENU(INPUT_INFO as *mut c_void)),
            Some(instance.into()),
            None,
        )?;
        state.borrow_mut().input_info = input_info;

        let specs = [
            ("STATIC", "Receiver name", WINDOW_STYLE::default()),
            ("EDIT", "", WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32)),
            ("STATIC", "Quality", WINDOW_STYLE::default()),
            ("COMBOBOX", "", WINDOW_STYLE(CBS_DROPDOWNLIST as u32)),
            ("BUTTON", "VSync", WINDOW_STYLE(BS_AUTOCHECKBOX as u32)),
            ("BUTTON", "Reconnect", WINDOW_STYLE(BS_AUTOCHECKBOX as u32)),
            ("BUTTON", "Apply", WINDOW_STYLE::default()),
        ];
        for (index, (class, title, style)) in specs.iter().enumerate() {
            let class = wide(class);
            let title = wide(title);
            let control = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_CHILD | WS_TABSTOP | *style,
                0,
                0,
                80,
                28,
                Some(hwnd),
                Some(HMENU((SETTINGS_FIRST + index) as *mut c_void)),
                Some(instance.into()),
                None,
            )?;
            state.borrow_mut().settings_controls.push(control);
        }
        let name = state.borrow().config.receiver_name.clone();
        label(state.borrow().settings_controls[1], &name);
        for quality in [
            "Auto",
            "Quality",
            "Balanced (30 FPS request)",
            "Low latency",
        ] {
            let text = wide(quality);
            SendMessageW(
                state.borrow().settings_controls[3],
                CB_ADDSTRING,
                None,
                Some(LPARAM(text.as_ptr() as isize)),
            );
        }
        let index = match state.borrow().config.quality {
            imirror_device::Quality::Auto => 0,
            imirror_device::Quality::Quality => 1,
            imirror_device::Quality::Balanced => 2,
            imirror_device::Quality::LowLatency => 3,
        };
        SendMessageW(
            state.borrow().settings_controls[3],
            CB_SETCURSEL,
            Some(WPARAM(index)),
            None,
        );
        for (index, checked) in [
            (4, state.borrow().config.vsync),
            (5, state.borrow().config.reconnect),
        ] {
            SendMessageW(
                state.borrow().settings_controls[index],
                BM_SETCHECK,
                Some(WPARAM(usize::from(checked))),
                None,
            );
        }
        label(
            state.borrow().controls[11],
            if state.borrow().config.one_to_one {
                "Fit"
            } else {
                "1:1"
            },
        );

        layout(hwnd, &state.borrow());
        SetTimer(Some(hwnd), 1, 200, None);
        let _ = ShowWindow(hwnd, if smoke_test { SW_HIDE } else { SW_SHOW });
        let direct_touch = std::env::args().any(|arg| arg == "--ble-direct-touch");
        if direct_touch || std::env::args().any(|arg| arg == "--ble-control") {
            let panel = crate::ble_panel::BlePanel::create(hwnd)?;
            panel.show();
            state.borrow_mut().ble_panel = Some(panel);
            state
                .borrow()
                .input
                .send(InputCommand::BleStart(if direct_touch {
                    imirror_input_ble::HidProfile::DirectTouch
                } else {
                    imirror_input_ble::HidProfile::RelativeMouse
                }));
        }
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 == -1 {
                state.borrow_mut().worker.stop();
                return Err(windows::core::Error::from_win32().into());
            }
            if result.0 == 0 {
                break;
            }
            if GetFocus() == state.borrow().preview || !IsDialogMessageW(hwnd, &message).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    Ok(())
}
fn layout(hwnd: HWND, state: &Ui) {
    // SAFETY: All HWNDs are UI-owned; GetClientRect initializes rect on success.
    unsafe {
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return;
        }
        let scale = GetDpiForWindow(hwnd) as f64 / 96.0;
        let px = |v: f64| (v * scale).round() as i32;
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let _ = MoveWindow(
            state.devices,
            px(8.0),
            px(8.0),
            (width - px(16.0)).max(1),
            px(250.0),
            true,
        );
        let columns = ((width - px(16.0)) / px(96.0)).max(1) as usize;
        let rows = state.controls.len().div_ceil(columns);
        for (index, control) in state.controls.iter().enumerate() {
            let _ = MoveWindow(
                *control,
                px(8.0 + (index % columns) as f64 * 96.0),
                px(40.0 + (index / columns) as f64 * 32.0),
                px(90.0),
                px(28.0),
                true,
            );
        }
        let top = 44.0 + rows as f64 * 32.0;

        if state.settings_controls.len() == 7 {
            let visibility = if state.settings_visible {
                SW_SHOW
            } else {
                SW_HIDE
            };
            for control in &state.settings_controls {
                let _ = ShowWindow(*control, visibility);
            }
            let inner = (width - px(16.0)).max(px(300.0));
            let _ = MoveWindow(
                state.settings_controls[0],
                px(8.0),
                px(top),
                px(108.0),
                px(24.0),
                true,
            );
            let _ = MoveWindow(
                state.settings_controls[1],
                px(120.0),
                px(top),
                inner - px(112.0),
                px(25.0),
                true,
            );
            let _ = MoveWindow(
                state.settings_controls[2],
                px(8.0),
                px(top + 31.0),
                px(108.0),
                px(24.0),
                true,
            );
            let _ = MoveWindow(
                state.settings_controls[3],
                px(120.0),
                px(top + 31.0),
                inner - px(112.0),
                px(180.0),
                true,
            );
            for (index, x, w) in [(4, 8.0, 85.0), (5, 100.0, 115.0), (6, 230.0, 80.0)] {
                let _ = MoveWindow(
                    state.settings_controls[index],
                    px(x),
                    px(top + 62.0),
                    px(w),
                    px(26.0),
                    true,
                );
            }
        }
        let top = top + if state.settings_visible { 96.0 } else { 0.0 };

        let _ = MoveWindow(
            state.ble_devices,
            px(8.0),
            px(top),
            (width - px(16.0)).max(1),
            px(180.0),
            true,
        );
        let _ = MoveWindow(
            state.input_info,
            px(8.0),
            px(top + 32.0),
            (width - px(16.0)).max(1),
            px(40.0),
            true,
        );
        let _ = MoveWindow(
            state.info,
            px(8.0),
            px(top + 76.0),
            (width - px(16.0)).max(1),
            px(54.0),
            true,
        );
        let video_top = px(top + 136.0);
        let _ = MoveWindow(
            state.preview,
            0,
            video_top,
            width,
            (height - video_top).max(1),
            true,
        );
    }
}
fn update(state: &mut Ui) {
    if let Ok(input) = state.input.snapshots.try_recv() {
        if input.ble_clients != state.input_snapshot.ble_clients {
            // SAFETY: Combo box is owned by this thread and copies strings.
            unsafe {
                SendMessageW(state.ble_devices, CB_RESETCONTENT, None, None);
                for client in &input.ble_clients {
                    let name = wide(client);
                    SendMessageW(
                        state.ble_devices,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(name.as_ptr() as isize)),
                    );
                }
            }
        }
        label(
            state.input_info,
            if input.message.is_empty() {
                "Control is off"
            } else {
                &input.message
            },
        );
        if let Some(panel) = &mut state.ble_panel {
            panel.update(&input);
        }
        // Keep the existing target selector in sync with observed selection.
        let index = input
            .ble
            .selected_target
            .as_ref()
            .and_then(|id| input.ble_clients.iter().position(|x| x == id));
        // SAFETY: The BLE combo is owned by this UI thread.
        unsafe {
            SendMessageW(
                state.ble_devices,
                CB_SETCURSEL,
                Some(WPARAM(index.unwrap_or(usize::MAX))),
                None,
            );
        }
        state.input_snapshot = input;
    }
    let Ok(snapshot) = state.worker.snapshots.try_recv() else {
        return;
    };
    let changed = state
        .snapshot
        .devices
        .iter()
        .map(|d| &d.id)
        .ne(snapshot.devices.iter().map(|d| &d.id));
    if changed {
        // SAFETY: Combo box lives on this thread and copies strings synchronously.
        unsafe {
            SendMessageW(
                state.devices,
                CB_RESETCONTENT,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            );
            for device in &snapshot.devices {
                let name = wide(&format!(
                    "{} — {} · iOS {}",
                    device.name, device.model, device.ios
                ));
                SendMessageW(
                    state.devices,
                    CB_ADDSTRING,
                    Some(WPARAM(0)),
                    Some(LPARAM(name.as_ptr() as isize)),
                );
            }
            SendMessageW(
                state.devices,
                CB_SETCURSEL,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            );
        }
    }
    if !snapshot.error.is_empty() {
        label(state.info, &snapshot.error);
    } else if snapshot.active {
        let s = &snapshot.status;
        label(
            state.info,
            &format!(
                "{}\r\nSource {} FPS · {} × {} · last decode {:.2} ms · {}",
                s.message,
                s.source_fps
                    .map(|fps| format!("{fps:.1}"))
                    .unwrap_or_else(|| "unavailable".into()),
                s.width,
                s.height,
                s.last_decode_ms,
                snapshot.decoder_mode
            ),
        );
    } else if snapshot.devices.is_empty() {
        label(
            state.info,
            "Connect your iPhone by USB, unlock it, and tap Trust This Computer.",
        );
    } else {
        label(
            state.info,
            "Select your iPhone and connect. USB capture also requires the supported capture filter.",
        );
    }
    if state.snapshot.status.width != snapshot.status.width
        || state.snapshot.status.height != snapshot.status.height
        || (state.snapshot.status.state == 4 && snapshot.status.state != 4)
    {
        release_input(state);
    }
    if state.input_snapshot.mode == 2
        && (state.snapshot.status.width != snapshot.status.width
            || state.snapshot.status.height != snapshot.status.height)
    {
        state.input_snapshot.geometry = None;
        state.input.send(InputCommand::RefreshGeometry);
    }
    state.snapshot = snapshot;
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: WM_NCCREATE contains a valid CREATESTRUCTW for this callback.
    // GWLP_USERDATA points to Ui owned by run(), valid until the message loop ends.
    unsafe {
        if msg == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }
        if msg == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        if msg == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Ui>;
        if !ptr.is_null() {
            // Do not retain this reference across dispatch of arbitrary messages.
            let Ok(mut borrowed) = (&*ptr).try_borrow_mut() else {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            };
            let state = &mut *borrowed;
            if (WM_APP..WM_APP + 0x400).contains(&msg) {
                video_input(hwnd, state, msg - WM_APP, wparam, lparam);
                return LRESULT(0);
            }
            match msg {
                WM_SIZE => {
                    release_input(state);
                    layout(hwnd, state);
                    return LRESULT(0);
                }
                WM_DPICHANGED => {
                    let suggested = &*(lparam.0 as *const RECT);
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    layout(hwnd, state);
                    return LRESULT(0);
                }
                WM_TIMER => {
                    update(state);
                    if state.smoke_test && state.started.elapsed().as_secs() >= 3 {
                        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                    return LRESULT(0);
                }
                WM_COMMAND => {
                    match wparam.0 & 0xffff {
                        FIT => {
                            release_input(state);
                            state.config.one_to_one = !state.config.one_to_one;
                            state.command(Command::Configure(
                                state.config.clone(),
                                state.preview.0 as usize,
                            ));
                            label(
                                state.controls[11],
                                if state.config.one_to_one {
                                    "Fit"
                                } else {
                                    "1:1"
                                },
                            );
                            if let Err(error) = crate::settings::save(&state.config) {
                                settings_error(hwnd, &error.to_string());
                            }
                        }
                        SETTINGS => {
                            release_input(state);
                            state.settings_visible = !state.settings_visible;
                            layout(hwnd, state);
                        }
                        SETTINGS_APPLY => {
                            let mut text = [0u16; 512];
                            let length = GetWindowTextW(state.settings_controls[1], &mut text)
                                .max(0) as usize;
                            let mut config = state.config.clone();
                            config.receiver_name = String::from_utf16_lossy(&text[..length]);
                            config.quality = match SendMessageW(
                                state.settings_controls[3],
                                CB_GETCURSEL,
                                None,
                                None,
                            )
                            .0
                            {
                                1 => imirror_device::Quality::Quality,
                                2 => imirror_device::Quality::Balanced,
                                3 => imirror_device::Quality::LowLatency,
                                _ => imirror_device::Quality::Auto,
                            };
                            config.vsync =
                                SendMessageW(state.settings_controls[4], BM_GETCHECK, None, None).0
                                    == 1;
                            config.reconnect =
                                SendMessageW(state.settings_controls[5], BM_GETCHECK, None, None).0
                                    == 1;
                            match crate::settings::save(&config) {
                                Ok(()) => {
                                    state.config = config;
                                    state.command(Command::Configure(
                                        state.config.clone(),
                                        state.preview.0 as usize,
                                    ));
                                    state.settings_visible = false;
                                    layout(hwnd, state);
                                }
                                Err(error) => settings_error(hwnd, &error.to_string()),
                            }
                        }

                        CONNECT => {
                            let selected = SendMessageW(
                                state.devices,
                                CB_GETCURSEL,
                                Some(WPARAM(0)),
                                Some(LPARAM(0)),
                            )
                            .0;
                            if selected >= 0
                                && let Some(device) = state.snapshot.devices.get(selected as usize)
                            {
                                state.command(Command::Connect(
                                    device.clone(),
                                    state.preview.0 as usize,
                                ));
                                label(state.info, "Connecting…");
                            }
                        }
                        DISCONNECT => {
                            release_input(state);
                            state.command(Command::Disconnect);
                        }
                        REFRESH => state.command(Command::Refresh),
                        ROTATE => {
                            release_input(state);
                            state.rotation = (state.rotation + 1) % 4;
                            state
                                .command(Command::Rotate(state.preview.0 as usize, state.rotation));
                        }
                        BLE | DIRECT_TOUCH => {
                            release_input(state);
                            let profile = if (wparam.0 & 0xffff) == DIRECT_TOUCH {
                                imirror_input_ble::HidProfile::DirectTouch
                            } else {
                                imirror_input_ble::HidProfile::RelativeMouse
                            };
                            if state.ble_panel.is_none() {
                                match crate::ble_panel::BlePanel::create(hwnd) {
                                    Ok(panel) => state.ble_panel = Some(panel),
                                    Err(error) => settings_error(hwnd, &error.to_string()),
                                }
                            }
                            if let Some(panel) = &mut state.ble_panel {
                                panel.update(&state.input_snapshot);
                                panel.show();
                            }
                            let wanted = if profile == imirror_input_ble::HidProfile::DirectTouch {
                                3
                            } else {
                                1
                            };
                            if state.input_snapshot.mode != wanted {
                                state.input.send(InputCommand::BleStart(profile));
                            }
                        }
                        crate::ble_panel::MOVE_RIGHT
                        | crate::ble_panel::MOVE_LEFT
                        | crate::ble_panel::LEFT_CLICK
                        | crate::ble_panel::TYPE_A => {
                            use imirror_input_ble::DiagnosticAction;
                            let action = match wparam.0 & 0xffff {
                                crate::ble_panel::MOVE_RIGHT => DiagnosticAction::MoveRight,
                                crate::ble_panel::MOVE_LEFT => DiagnosticAction::MoveLeft,
                                crate::ble_panel::LEFT_CLICK => DiagnosticAction::LeftClick,
                                _ => DiagnosticAction::TypeA,
                            };
                            state.input.send(InputCommand::BleDiagnostic(action));
                        }
                        crate::ble_panel::SET_SPEED => {
                            if (imirror_input_core::MIN_POINTER_SPEED as isize
                                ..=imirror_input_core::MAX_POINTER_SPEED as isize)
                                .contains(&lparam.0)
                            {
                                state
                                    .input
                                    .send(InputCommand::PointerSpeed(lparam.0 as u16));
                            }
                        }
                        crate::ble_panel::SELECT_TARGET => {
                            if lparam.0 >= 0
                                && let Some(id) =
                                    state.input_snapshot.ble_clients.get(lparam.0 as usize)
                            {
                                state.input.send(InputCommand::BleSelect(id.clone()));
                            }
                        }
                        WDA => {
                            release_input(state);
                            state.input.send(InputCommand::WdaConnect);
                        }
                        DISABLE => {
                            release_input(state);
                            state.input.send(InputCommand::Disable);
                        }
                        HOME => {
                            state
                                .input
                                .send(InputCommand::Action(Input::Button(Button::Home)));
                        }
                        AIRPLAY => {
                            release_input(state);
                            state.command(Command::AirPlay(state.preview.0 as usize));
                        }
                        FULL => fullscreen(hwnd, state),
                        BLE_DEVICE if (wparam.0 >> 16) as u32 == CBN_SELCHANGE => {
                            let index = SendMessageW(state.ble_devices, CB_GETCURSEL, None, None).0;
                            if index >= 0
                                && let Some(id) =
                                    state.input_snapshot.ble_clients.get(index as usize)
                            {
                                state.input.send(InputCommand::BleSelect(id.clone()));
                            }
                        }
                        _ => {}
                    }
                    return LRESULT(0);
                }
                WM_CLOSE => {
                    let _ = KillTimer(Some(hwnd), 1);
                    state.input.stop();
                    state.worker.stop();
                    let _ = DestroyWindow(hwnd);
                    return LRESULT(0);
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system" fn preview_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Windows supplies valid handles and scalar event data. Messages are
    // posted so parent input routing never borrows Ui inside a reentrant callback.
    unsafe {
        if matches!(
            msg,
            WM_LBUTTONDOWN
                | WM_LBUTTONUP
                | WM_MOUSEMOVE
                | WM_MOUSEWHEEL
                | WM_KEYDOWN
                | WM_KEYUP
                | WM_CHAR
                | WM_KILLFOCUS
                | WM_CAPTURECHANGED
        ) {
            if msg == WM_LBUTTONDOWN {
                let _ = SetFocus(Some(hwnd));
                SetCapture(hwnd);
            }
            if let Ok(parent) = GetParent(hwnd) {
                let _ = PostMessageW(Some(parent), WM_APP + msg, wparam, lparam);
            }
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

struct WindowLifetime<'a> {
    hwnd: HWND,
    state: &'a RefCell<Ui>,
}
impl Drop for WindowLifetime<'_> {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.input.stop();
        state.worker.stop();
        // SAFETY: Workers have stopped using HWNDs. Clear userdata before final
        // destruction so even an early initialization error cannot leave a stale pointer.
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
fn release_input(state: &mut Ui) {
    state.gesture.cancel();
    state.captured = false;
    state.last_mouse = None;
    state.keys.clear();
    state.input.cancel();
}
fn mapped(state: &Ui, p: Point) -> Option<Point> {
    if state.snapshot.status.state != 4 {
        return None;
    }
    let size = if state.input_snapshot.mode == 3 {
        Size {
            width: 10000.0,
            height: 10000.0,
        }
    } else if state.input_snapshot.mode == 2 {
        state.input_snapshot.geometry?
    } else {
        Size {
            width: state.snapshot.status.width as f64,
            height: state.snapshot.status.height as f64,
        }
    };
    let mut bounds = RECT::default();
    // SAFETY: Preview is owned by this UI thread; bounds is writable.
    unsafe {
        GetClientRect(state.preview, &mut bounds).ok()?;
    }
    let mapper = Mapper::new(
        Rect {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: bounds.right as f64,
                height: bounds.bottom as f64,
            },
        },
        Size {
            width: state.snapshot.status.width as f64,
            height: state.snapshot.status.height as f64,
        },
        size,
        Rotation::from_quarters(state.rotation as u32),
        Rotation::R0,
        if state.config.one_to_one {
            ScaleMode::OneToOne
        } else {
            ScaleMode::Fit
        },
    )
    .ok()?;
    mapper.map(p)
}
fn hid_key(vk: usize) -> Option<u8> {
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
fn video_input(hwnd: HWND, state: &mut Ui, msg: u32, wparam: WPARAM, lparam: LPARAM) {
    let p = Point {
        x: (lparam.0 as u16 as i16) as f64,
        y: ((lparam.0 >> 16) as u16 as i16) as f64,
    };
    // SAFETY: GetKeyState and capture/window calls operate on this UI thread.
    unsafe {
        let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
        if msg == WM_KEYDOWN && wparam.0 == 0x46 && ctrl && shift {
            release_input(state);
            fullscreen(hwnd, state);
            return;
        }
        if msg == WM_KILLFOCUS || (msg == WM_KEYDOWN && wparam.0 == 0x1b) {
            release_input(state);
            let _ = ReleaseCapture();
            return;
        }
        if msg == WM_LBUTTONDOWN
            && state.input_snapshot.mode == 3
            && !state.input_snapshot.ble.touch_ready()
        {
            release_input(state);
            let _ = ReleaseCapture();
            return;
        }
        if msg == WM_LBUTTONDOWN {
            if let Some(point) = mapped(state, p) {
                state.captured = true;
                state.last_mouse = Some(p);
                if state.input_snapshot.mode == 2 {
                    state.gesture.press(Some(point), Instant::now());
                }
                if state.input_snapshot.mode == 1 {
                    state.input.send(InputCommand::Mouse(1, 0, 0, 0));
                } else if state.input_snapshot.mode == 3 {
                    state.input.send(InputCommand::TouchContact(
                        point.x.round() as i32,
                        point.y.round() as i32,
                    ));
                }
            } else {
                release_input(state);
                let _ = ReleaseCapture();
            }
        } else if msg == WM_LBUTTONUP {
            if state.input_snapshot.mode == 3 {
                state.input.send(InputCommand::TouchRelease);
            } else if state.input_snapshot.mode == 2 {
                if let Some(action) = state.gesture.release(mapped(state, p), Instant::now()) {
                    state.input.send(InputCommand::Action(action));
                }
            } else if state.captured {
                state.input.send(InputCommand::Mouse(0, 0, 0, 0));
            }
            let _ = ReleaseCapture();
        } else if msg == WM_MOUSEMOVE
            && state.captured
            && state.input_snapshot.mode == 3
            && wparam.0 & 1 != 0
        {
            if let Some(point) = mapped(state, p) {
                state.input.send(InputCommand::TouchContact(
                    point.x.round() as i32,
                    point.y.round() as i32,
                ));
            } else {
                release_input(state);
                let _ = ReleaseCapture();
            }
        } else if msg == WM_MOUSEMOVE && state.captured && state.input_snapshot.mode == 1 {
            if mapped(state, p).is_none() {
                release_input(state);
                return;
            }
            if let Some(last) = state.last_mouse.replace(p) {
                state.input.send(InputCommand::Mouse(
                    if wparam.0 & 1 != 0 { 1 } else { 0 },
                    (p.x - last.x) as i32,
                    (p.y - last.y) as i32,
                    0,
                ));
            }
        } else if msg == WM_MOUSEWHEEL && state.captured {
            let mut client_point = windows::Win32::Foundation::POINT {
                x: p.x as i32,
                y: p.y as i32,
            };
            let _ = windows::Win32::Graphics::Gdi::ScreenToClient(state.preview, &mut client_point);
            if mapped(
                state,
                Point {
                    x: client_point.x as f64,
                    y: client_point.y as f64,
                },
            )
            .is_none()
            {
                return;
            }
            let delta = ((wparam.0 >> 16) as u16 as i16) as i32 / 120;
            if state.input_snapshot.mode == 1 {
                state.input.send(InputCommand::Mouse(0, 0, 0, delta));
            } else if let Some(size) = state.input_snapshot.geometry {
                let from = Point {
                    x: size.width * 0.5,
                    y: size.height * 0.5,
                };
                let to = if shift {
                    Point {
                        x: (from.x + delta as f64 * size.width * 0.2).clamp(1.0, size.width - 1.0),
                        y: from.y,
                    }
                } else {
                    Point {
                        x: from.x,
                        y: (from.y + delta as f64 * size.height * 0.2)
                            .clamp(1.0, size.height - 1.0),
                    }
                };
                state.input.send(InputCommand::Action(Input::Swipe {
                    from,
                    to,
                    duration: std::time::Duration::from_millis(180),
                }));
            }
        } else if (msg == WM_KEYDOWN || msg == WM_KEYUP)
            && state.captured
            && matches!(state.input_snapshot.mode, 1 | 3)
        {
            if let Some(key) = hid_key(wparam.0) {
                if msg == WM_KEYDOWN && !state.keys.contains(&key) {
                    state.keys.push(key);
                }
                if msg == WM_KEYUP {
                    state.keys.retain(|v| *v != key);
                }
            }
            let modifiers = u8::from(ctrl)
                | (u8::from(shift) << 1)
                | (u8::from(GetKeyState(VK_MENU.0 as i32) < 0) << 2);
            state
                .input
                .send(InputCommand::Key(modifiers, state.keys.clone()));
        } else if msg == WM_CHAR && state.captured && state.input_snapshot.mode == 2 {
            let unit = wparam.0 as u16;
            if (0xd800..=0xdbff).contains(&unit) {
                state.surrogate = Some(unit);
                return;
            }
            let text = if let Some(high) = state.surrogate.take() {
                String::from_utf16_lossy(&[high, unit])
            } else {
                String::from_utf16_lossy(&[unit])
            };
            state.input.send(InputCommand::Action(Input::Text(text)));
        }
    }
}
fn fullscreen(hwnd: HWND, state: &mut Ui) {
    // SAFETY: Window/monitor handles are live; style changes and geometry changes
    // happen on the UI thread and preserve the previous window rectangle.
    unsafe {
        if let Some(rect) = state.fullscreen.take() {
            SetWindowLongPtrW(
                hwnd,
                GWL_STYLE,
                (WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE).0 as isize,
            );
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        } else {
            let mut original = RECT::default();
            if GetWindowRect(hwnd, &mut original).is_err() {
                return;
            }
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return;
            }
            state.fullscreen = Some(original);
            SetWindowLongPtrW(
                hwnd,
                GWL_STYLE,
                (WS_POPUP | WS_CLIPCHILDREN | WS_VISIBLE).0 as isize,
            );
            let r = info.rcMonitor;
            let _ = SetWindowPos(
                hwnd,
                None,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
        layout(hwnd, state);
    }
}

fn settings_error(hwnd: HWND, message: &str) {
    let text = wide(message);
    // SAFETY: The parent HWND and copied NUL-terminated text are live on the UI thread.
    unsafe {
        MessageBoxW(
            Some(hwnd),
            PCWSTR(text.as_ptr()),
            w!("Settings could not be saved"),
            MB_OK | MB_ICONWARNING,
        );
    }
}
