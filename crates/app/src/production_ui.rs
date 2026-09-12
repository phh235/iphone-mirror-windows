//! Compact native window. The existing worker exclusively owns video transport/decoder/renderer.
use crate::{
    ble_panel,
    control::{self, Backend, ControlManager},
    settings_window as settings, theme,
    worker::{self, Worker},
};
use imirror_coordinate_map::{Mapper, Point, Rect, Rotation, ScaleMode, Size};
use imirror_device::{Config, ConnectionChoice, ControlChoice, DisplayChoice};
use imirror_input_core::{Button, Gesture, Input};
use std::{
    cell::RefCell,
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use windows::{
    Foundation::TypedEventHandler,
    UI::ViewManagement::UISettings,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::{
            LibraryLoader::GetModuleHandleW,
            SystemServices::{SS_CENTER, SS_ENDELLIPSIS},
        },
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::{PCWSTR, w},
};
const CONNECT: usize = 101;
const ROTATE: usize = 105;
const CONTROL: usize = 108;
const HOME: usize = 111;
const FULL: usize = 112;
const SETTINGS: usize = 117;
const DEVICE_LABEL: usize = 118;
const STATUS_LABEL: usize = 119;
const EMPTY: usize = 120;
const THEME_CHANGED: u32 = WM_APP + 71;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn label(window: HWND, s: &str) {
    let text = wide(s); // SAFETY: UI-owned handle; native control copies the text synchronously.
    unsafe {
        let mut current = [0u16; 256];
        let n = GetWindowTextW(window, &mut current).max(0) as usize;
        if current[..n] != text[..text.len() - 1] {
            let _ = SetWindowTextW(window, PCWSTR(text.as_ptr()));
        }
    }
}
struct AppearanceWatch {
    settings: UISettings,
    token: i64,
    alive: Arc<AtomicBool>,
}
impl AppearanceWatch {
    fn new(window: HWND) -> windows::core::Result<Self> {
        let settings = UISettings::new()?;
        let target = window.0 as usize;
        let alive = Arc::new(AtomicBool::new(true));
        let active = alive.clone();
        let token = settings.ColorValuesChanged(&TypedEventHandler::new(move |_, _| {
            if active.load(Ordering::Acquire) {
                // SAFETY: Callback only posts a scalar message; alive is cleared before HWND destruction.
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(target as *mut c_void)),
                        THEME_CHANGED,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
            Ok(())
        }))?;
        Ok(Self {
            settings,
            token,
            alive,
        })
    }
}
impl Drop for AppearanceWatch {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
        let _ = self.settings.RemoveColorValuesChanged(self.token);
    }
}
struct Ui {
    window: HWND,
    host: HWND,
    preview: HWND,
    device_label: HWND,
    status_label: HWND,
    empty: HWND,
    connect: HWND,
    rotate: HWND,
    control_button: HWND,
    home: HWND,
    full: HWND,
    settings_button: HWND,
    worker: Worker,
    video: worker::Snapshot,
    control: ControlManager,
    input: control::Snapshot,
    config: Config,
    settings: Option<settings::Panel>,
    diagnostics: Option<ble_panel::BlePanel>,
    selected: usize,
    wanted: bool,
    rotation: i32,
    fullscreen: Option<RECT>,
    gesture: Gesture,
    surrogate: Option<u16>,
    smoke: bool,
    started: Instant,
    appearance: Option<AppearanceWatch>,
    last_geometry: (u32, u32),
    last_theme_refresh: Instant,
}
struct WindowLifetime<'a> {
    window: HWND,
    state: &'a RefCell<Ui>,
}
impl Drop for WindowLifetime<'_> {
    fn drop(&mut self) {
        {
            let mut ui = self.state.borrow_mut();
            ui.appearance = None;
            ui.control.stop();
            ui.worker.stop();
            ui.settings = None;
            ui.diagnostics = None;
        }
        // SAFETY: Workers have released the preview; userdata is cleared before destroying the UI.
        unsafe {
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.window);
        }
        theme::forget(self.window);
    }
}
impl Ui {
    fn command(&self, command: worker::Command) {
        if self.worker.commands.try_send(command).is_err() {
            label(self.status_label, "A connection operation is in progress");
        }
    }
    fn connect_device(&mut self) {
        if self.config.connection == ConnectionChoice::Wireless {
            self.command(worker::Command::AirPlay(self.preview.0 as usize));
            self.wanted = false;
        } else if let Some(device) = self
            .video
            .devices
            .get(self.selected)
            .or_else(|| self.video.devices.first())
            .cloned()
        {
            self.command(worker::Command::Connect(device, self.preview.0 as usize));
            self.wanted = false;
        } else {
            self.wanted = true;
            label(
                self.status_label,
                "Connect your iPhone by USB and unlock it",
            );
        }
    }
    fn persist(&self) {
        if let Err(error) = crate::settings::save(&self.config) {
            self.error(&format!("Could not save settings: {error}"));
        }
    }
    fn error(&self, text: &str) {
        let text = wide(text); // SAFETY: UI-owned modal error box, copied UTF-16 strings.
        unsafe {
            MessageBoxW(
                Some(self.window),
                PCWSTR(text.as_ptr()),
                w!("iMirror"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    fn open_settings(&mut self, page: usize) {
        self.control.release();
        if self.settings.is_none() {
            match settings::Panel::create(self.window) {
                Ok(panel) => {
                    panel.set_receiver_name(&self.config.receiver_name);
                    self.settings = Some(panel);
                }
                Err(e) => {
                    self.error(&e.to_string());
                    return;
                }
            }
        }
        if let Some(panel) = &mut self.settings {
            panel.update(&self.config, &self.input, &self.video.devices);
            panel.show(page);
        }
    }
    fn open_diagnostics(&mut self) {
        self.control.release();
        if self.diagnostics.is_none() {
            match ble_panel::BlePanel::create(self.window) {
                Ok(panel) => self.diagnostics = Some(panel),
                Err(e) => {
                    self.error(&e.to_string());
                    return;
                }
            }
        }
        if let Some(panel) = &mut self.diagnostics {
            panel.update(&self.input);
            panel.show();
        }
    }
    fn set_control(&mut self, enabled: bool) {
        self.config.control_enabled = enabled;
        if !enabled {
            self.control.disable();
        } else if self.config.advanced && self.config.control == ControlChoice::Wda {
            self.control.send(control::Command::WdaConnect);
        } else if !self.control.enable() {
            self.config.control_enabled = false;
            self.error("Mouse control is unavailable. Open Advanced Diagnostics for details; Ctrl+Alt+Q must be available before capture.");
        }
        self.persist();
        self.refresh_labels();
    }
    fn refresh_labels(&self) {
        let device = self
            .video
            .devices
            .get(self.selected)
            .or_else(|| self.video.devices.first());
        label(
            self.device_label,
            device.map(|d| d.name.as_str()).unwrap_or("iPhone"),
        );
        label(
            self.status_label,
            if self.video.status.state == 4 {
                "Connected"
            } else if self.video.active {
                "Connecting…"
            } else if !self.video.error.is_empty() {
                "Connection unavailable — open Settings"
            } else {
                "Not connected"
            },
        );
        label(
            self.connect,
            if self.video.active {
                "Disconnect"
            } else {
                "Connect"
            },
        );
        label(
            self.control_button,
            if self.control.mailbox.captured.load(Ordering::Acquire) {
                "Control active"
            } else if self.config.control_enabled {
                "Control on"
            } else {
                "Control"
            },
        );
        theme::set_active(self.control_button, self.config.control_enabled);
        let capabilities = self.control.capabilities();
        // SAFETY: This thread owns each control. Unsupported actions are hidden rather than silently ignored.
        unsafe {
            let _ = ShowWindow(
                self.host,
                if self.video.status.state == 4 {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let _ = ShowWindow(
                self.home,
                if capabilities.home && self.input.geometry.is_some() {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let _ = EnableWindow(self.rotate, self.video.status.state == 4);
            let _ = EnableWindow(
                self.control_button,
                capabilities.mouse || self.control.backend() == Backend::Wda,
            );
            let _ = ShowWindow(
                self.empty,
                if self.video.status.state == 4 {
                    SW_HIDE
                } else {
                    SW_SHOW
                },
            );
            let _ = EnableWindow(self.home, capabilities.home && capabilities.keyboard);
        }
        label(
            self.empty,
            if self.config.connection == ConnectionChoice::Wireless {
                "On iPhone, open Screen Mirroring and choose this PC."
            } else {
                "Connect your iPhone by USB and unlock it.\nChoose Wireless in Settings to connect over Wi-Fi."
            },
        );
    }
    fn update(&mut self) {
        if let Ok(input) = self.control.snapshots.try_recv() {
            self.input = input;
        }
        if let Some(panel) = &mut self.diagnostics {
            panel.update(&self.input);
        }
        if let Ok(video) = self.worker.snapshots.try_recv() {
            if self.video.active && !video.active {
                self.control.release();
            }
            self.video = video;
            if !self.smoke && self.wanted && !self.video.active && !self.video.devices.is_empty() {
                self.connect_device();
            }
            let geometry = (self.video.status.width, self.video.status.height);
            if geometry != self.last_geometry {
                self.last_geometry = geometry;
                self.control.send(control::Command::RefreshGeometry);
                layout(self);
            }
        }
        self.refresh_labels();
        if let Some(panel) = &mut self.settings {
            panel.update(&self.config, &self.input, &self.video.devices);
        }
        // WM_SETTINGCHANGE and UISettings are primary; a low-rate fallback handles missed broadcasts.
        if self.last_theme_refresh.elapsed().as_secs() >= 30 {
            self.last_theme_refresh = Instant::now();
            theme::refresh(self.window);
        }
        if self.smoke && self.started.elapsed().as_millis() > 1500 {
            let args: Vec<_> = std::env::args().collect();
            if let Some(index) = args.iter().position(|a| a == "--ui-snapshot")
                && let Some(path) = args.get(index + 1)
            {
                let target = self
                    .settings
                    .as_ref()
                    .map(settings::Panel::handle)
                    .unwrap_or(self.window);
                if let Err(error) = crate::ui_snapshot::save(target, std::path::Path::new(path)) {
                    eprintln!("UI snapshot failed: {error}");
                }
            }
            // SAFETY: Scalar close request to our own window.
            unsafe {
                let _ = PostMessageW(Some(self.window), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
    fn settings_action(&mut self, id: usize, value: isize) {
        let mut configure = false;
        match id {
            settings::AUTO | settings::USB | settings::WIRELESS => {
                self.control.release();
                self.config.connection = match id {
                    settings::USB => ConnectionChoice::Usb,
                    settings::WIRELESS => ConnectionChoice::Wireless,
                    _ => ConnectionChoice::Automatic,
                };
                if self.video.active {
                    self.command(worker::Command::Disconnect);
                }
                self.wanted = self.config.connection == ConnectionChoice::Automatic;
            }
            settings::DEVICE => {
                if value >= 0 {
                    self.selected = value as usize;
                }
            }
            settings::REFRESH => self.command(worker::Command::Refresh),
            settings::RECEIVER => {
                if let Some(panel) = &self.settings {
                    let name = panel.receiver_name();
                    if !name.trim().is_empty() {
                        self.config.receiver_name = name;
                        configure = true;
                    }
                }
            }
            settings::CONTROL => {
                self.set_control(value != 0);
                return;
            }
            settings::CONTROL_AUTO | settings::CONTROL_BT | settings::CONTROL_WDA => {
                self.config.control = match id {
                    settings::CONTROL_BT => ControlChoice::BluetoothMouse,
                    settings::CONTROL_WDA if self.config.advanced => ControlChoice::Wda,
                    _ => ControlChoice::Automatic,
                };
                if self.config.control_enabled {
                    self.set_control(true);
                }
            }
            settings::SPEED => {
                self.control
                    .send(control::Command::PointerSpeed(value.clamp(5, 200) as u16));
                return;
            }
            settings::FIT | settings::ONE | settings::FILL => {
                self.control.release();
                self.config.display = match id {
                    settings::ONE => DisplayChoice::OneToOne,
                    settings::FILL => DisplayChoice::Fill,
                    _ => DisplayChoice::Fit,
                };
                self.config.one_to_one = self.config.display == DisplayChoice::OneToOne;
                configure = true;
                layout(self);
            }
            settings::VSYNC => {
                self.config.vsync = value != 0;
                configure = true;
            }
            settings::ADVANCED => {
                self.config.advanced = value != 0;
                if !self.config.advanced && self.config.control == ControlChoice::Wda {
                    self.config.control = ControlChoice::Automatic;
                    if self.config.control_enabled {
                        self.set_control(true);
                    }
                }
            }
            settings::DIAGNOSTICS => {
                self.open_diagnostics();
                return;
            }
            settings::WDA if self.config.advanced => {
                self.config.control = ControlChoice::Wda;
                self.set_control(true);
                return;
            }
            _ => return,
        }
        self.persist();
        if configure {
            self.command(worker::Command::Configure(
                self.config.clone(),
                self.preview.0 as usize,
            ));
        }
        if let Some(panel) = &mut self.settings {
            panel.update(&self.config, &self.input, &self.video.devices);
        }
    }
}
pub fn run(smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    let _mta = imirror_platform_windows::Mta::new()?;
    // SAFETY: Set once before creating windows; an embedded PerMonitorV2 manifest may have set it already.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let config = crate::settings::load()?;
    let state = Box::new(RefCell::new(Ui {
        window: HWND::default(),
        host: HWND::default(),
        preview: HWND::default(),
        device_label: HWND::default(),
        status_label: HWND::default(),
        empty: HWND::default(),
        connect: HWND::default(),
        rotate: HWND::default(),
        control_button: HWND::default(),
        home: HWND::default(),
        full: HWND::default(),
        settings_button: HWND::default(),
        worker: Worker::start(config.clone())?,
        video: worker::Snapshot::default(),
        control: ControlManager::start()?,
        input: control::Snapshot::default(),
        wanted: config.connection == ConnectionChoice::Automatic,
        config,
        settings: None,
        diagnostics: None,
        selected: 0,
        rotation: 0,
        fullscreen: None,
        gesture: Gesture::default(),
        surrogate: None,
        smoke,
        started: Instant::now(),
        appearance: None,
        last_geometry: (0, 0),
        last_theme_refresh: Instant::now(),
    }));
    // SAFETY: All HWND creation/dispatch stays on this thread; state allocation is stable until worker shutdown and window destruction.
    unsafe {
        let module = GetModuleHandleW(None)?;
        for (name, proc, brush) in [
            (
                w!("iMirrorWindow"),
                Some(
                    window_proc as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
                ),
                HBRUSH::default(),
            ),
            (
                w!("iMirrorVideoHost"),
                Some(host_proc as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT),
                HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ),
            (
                w!("iMirrorVideo"),
                Some(
                    preview_proc as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
                ),
                HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ),
        ] {
            let class = WNDCLASSW {
                lpfnWndProc: proc,
                hInstance: module.into(),
                lpszClassName: name,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: brush,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_win32().into());
            }
        }
        let scale = GetDpiForSystem() as f64 / 96.0;
        let mut work = RECT::default();
        let work_known = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut work as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        )
        .is_ok();
        let initial_width = if work_known {
            (620.0 * scale).min((work.right - work.left - 32).max(1) as f64)
        } else {
            620.0 * scale
        };
        let initial_height = if work_known {
            (900.0 * scale).min((work.bottom - work.top - 32).max(1) as f64)
        } else {
            900.0 * scale
        };
        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("iMirrorWindow"),
            w!("iMirror"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            initial_width as i32,
            initial_height as i32,
            None,
            None,
            Some(module.into()),
            Some((&*state as *const RefCell<Ui>).cast()),
        )?;
        state.borrow_mut().window = window;
        let _lifetime = WindowLifetime {
            window,
            state: &state,
        };
        let make = |id: usize,
                    title: &str,
                    class: PCWSTR,
                    style: WINDOW_STYLE,
                    parent: HWND|
         -> windows::core::Result<HWND> {
            let text = wide(title);
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                PCWSTR(text.as_ptr()),
                WS_CHILD | WS_VISIBLE | style,
                0,
                0,
                120,
                36,
                Some(parent),
                Some(HMENU(id as *mut c_void)),
                Some(module.into()),
                None,
            )
        };
        {
            let mut ui = state.borrow_mut();
            ui.device_label = make(
                DEVICE_LABEL,
                "iPhone",
                w!("STATIC"),
                WINDOW_STYLE(SS_ENDELLIPSIS.0),
                window,
            )?;
            ui.status_label = make(
                STATUS_LABEL,
                "Not connected",
                w!("STATIC"),
                WINDOW_STYLE(SS_ENDELLIPSIS.0),
                window,
            )?;
            let button = WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32);
            ui.connect = make(CONNECT, "Connect", w!("BUTTON"), button, window)?;
            ui.settings_button = make(SETTINGS, "Settings", w!("BUTTON"), button, window)?;
            ui.full = make(FULL, "Fullscreen", w!("BUTTON"), button, window)?;
            ui.home = make(HOME, "Home", w!("BUTTON"), button, window)?;
            ui.rotate = make(ROTATE, "Rotate", w!("BUTTON"), button, window)?;
            ui.control_button = make(CONTROL, "Control", w!("BUTTON"), button, window)?;
            for child in [
                ui.connect,
                ui.settings_button,
                ui.full,
                ui.home,
                ui.rotate,
                ui.control_button,
            ] {
                theme::style_button(child);
            }
            ui.host = make(
                0,
                "",
                w!("iMirrorVideoHost"),
                WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                window,
            )?;
            ui.preview = make(
                0,
                "",
                w!("iMirrorVideo"),
                WS_CLIPSIBLINGS | WS_TABSTOP,
                ui.host,
            )?;
            ui.empty = make(
                EMPTY,
                "Connect your iPhone",
                w!("STATIC"),
                WINDOW_STYLE(SS_CENTER.0),
                window,
            )?;
            ui.appearance = AppearanceWatch::new(window).ok();
        }
        let appearance = theme::refresh(window);
        appearance.set_font(state.borrow().device_label, true);
        layout(&state.borrow());
        state.borrow().refresh_labels();
        SetTimer(Some(window), 1, 200, None);
        let _ = ShowWindow(window, if smoke { SW_HIDE } else { SW_SHOWNORMAL });
        if smoke && std::env::args().any(|a| a == "--ui-snapshot") {
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            let _ = UpdateWindow(window);
        }
        if !smoke {
            let mut ui = state.borrow_mut();
            if ui.config.control_enabled {
                ui.set_control(true);
            }
            if std::env::args().any(|a| a == "--ble-control") {
                ui.set_control(true);
            }
        }
        if smoke {
            let args: Vec<_> = std::env::args().collect();
            if let Some(index) = args.iter().position(|a| a == "--ui-settings-page")
                && let Some(page) = args.get(index + 1).and_then(|p| p.parse::<usize>().ok())
            {
                state.borrow_mut().open_settings(page);
            }
        }
        let mut message = MSG::default();
        let mut message_error = None;
        loop {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 < 0 {
                message_error = Some(windows::core::Error::from_win32());
                break;
            }
            if result.0 == 0 {
                break;
            }
            let panel = state
                .borrow()
                .settings
                .as_ref()
                .map(settings::Panel::handle);
            let focus = GetFocus();
            let in_settings = panel.is_some_and(|panel| {
                IsWindowVisible(panel).as_bool()
                    && (focus == panel || IsChild(panel, focus).as_bool())
                    && IsDialogMessageW(panel, &message).as_bool()
            });
            let preview = state.borrow().preview;
            let in_main =
                !in_settings && focus != preview && IsDialogMessageW(window, &message).as_bool();
            if !in_settings && !in_main {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if let Some(error) = message_error {
            return Err(error.into());
        }
    }
    Ok(())
}
fn layout(ui: &Ui) {
    if ui.preview.0.is_null() {
        return;
    }
    let appearance = theme::current(ui.window);
    let px = |n| appearance.px(n);
    // SAFETY: All handles are owned by the main UI; the native video worker retains the same preview HWND throughout.
    unsafe {
        let mut bounds = RECT::default();
        if GetClientRect(ui.window, &mut bounds).is_err() {
            return;
        }
        let width = bounds.right;
        let height = bounds.bottom;
        let top = px(64);
        let bottom = px(64);
        for (window, x, y, w, h) in [
            (
                ui.device_label,
                px(16),
                px(9),
                (width - px(322)).max(px(64)),
                px(25),
            ),
            (
                ui.status_label,
                px(16),
                px(35),
                (width - px(322)).max(px(64)),
                px(22),
            ),
            (ui.connect, width - px(298), px(14), px(90), px(36)),
            (ui.settings_button, width - px(200), px(14), px(86), px(36)),
            (ui.full, width - px(106), px(14), px(90), px(36)),
        ] {
            let _ = MoveWindow(window, x, y, w, h, true);
        }
        let viewport_width = width.max(1);
        let viewport_height = (height - top - bottom).max(1);
        let _ = MoveWindow(ui.host, 0, top, viewport_width, viewport_height, true);
        let (mut source_width, mut source_height) =
            (ui.video.status.width as f64, ui.video.status.height as f64);
        if ui.rotation % 2 != 0 {
            std::mem::swap(&mut source_width, &mut source_height);
        }
        let (x, y, w, h) = if ui.config.display == DisplayChoice::Fill
            && source_width > 0.0
            && source_height > 0.0
        {
            let scale =
                (viewport_width as f64 / source_width).max(viewport_height as f64 / source_height);
            let w = (source_width * scale).ceil() as i32;
            let h = (source_height * scale).ceil() as i32;
            ((viewport_width - w) / 2, (viewport_height - h) / 2, w, h)
        } else {
            (0, 0, viewport_width, viewport_height)
        };
        let _ = MoveWindow(ui.preview, x, y, w, h, true);
        let home = ui.control.capabilities().home && ui.input.geometry.is_some();
        let count = if home { 3 } else { 2 };
        let total = px(count * 104 + (count - 1) * 8);
        let start = (width - total) / 2;
        let y = (height - bottom) + (bottom - px(36)) / 2;
        let mut index = 0;
        if home {
            let _ = MoveWindow(ui.home, start, y, px(104), px(36), true);
            index += 1;
        }
        for control in [ui.rotate, ui.control_button] {
            let _ = MoveWindow(control, start + index * px(112), y, px(104), px(36), true);
            index += 1;
        }
        let _ = MoveWindow(
            ui.empty,
            px(32),
            top + (viewport_height - px(88)) / 2,
            (width - px(64)).max(1),
            px(88),
            true,
        );
    }
}
fn map(ui: &Ui, p: Point) -> Option<Point> {
    if ui.video.status.state != 4 {
        return None;
    }
    let mut rect = RECT::default(); // SAFETY: UI-owned preview rectangle output.
    unsafe {
        GetClientRect(ui.preview, &mut rect).ok()?;
    }
    let stream = Size {
        width: ui.video.status.width as f64,
        height: ui.video.status.height as f64,
    };
    let device = if ui.control.backend() == Backend::Wda {
        ui.input.geometry?
    } else {
        stream
    };
    Mapper::new(
        Rect {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: rect.right as f64,
                height: rect.bottom as f64,
            },
        },
        stream,
        device,
        Rotation::from_quarters(ui.rotation as u32),
        Rotation::R0,
        if ui.config.display == DisplayChoice::OneToOne {
            ScaleMode::OneToOne
        } else {
            ScaleMode::Fit
        },
    )
    .ok()?
    .map(p)
}
fn preview_input(ui: &mut Ui, message: u32, wparam: WPARAM, lparam: LPARAM) {
    let point = Point {
        x: (lparam.0 as u16 as i16) as f64,
        y: ((lparam.0 >> 16) as u16 as i16) as f64,
    };
    // SAFETY: Input capture/focus geometry operations refer only to this UI's windows.
    unsafe {
        if message == WM_KILLFOCUS || (message == WM_KEYDOWN && wparam.0 == 0x1b) {
            ui.control.release();
            ui.gesture.cancel();
            let _ = ReleaseCapture();
            return;
        }
        if message == WM_KEYDOWN
            && wparam.0 == 0x46
            && GetKeyState(VK_CONTROL.0 as i32) < 0
            && GetKeyState(VK_SHIFT.0 as i32) < 0
        {
            ui.control.release();
            fullscreen(ui);
            return;
        }
        if message == WM_LBUTTONDOWN && ui.config.control_enabled && ui.control.is_ready() {
            if let Some(mapped) = map(ui, point) {
                if ui.control.backend() == Backend::BluetoothMouse {
                    let mut rect = RECT::default();
                    let mut origin = windows::Win32::Foundation::POINT::default();
                    if GetClientRect(ui.host, &mut rect).is_ok()
                        && ClientToScreen(ui.host, &mut origin).as_bool()
                    {
                        rect.left += origin.x;
                        rect.right += origin.x;
                        rect.top += origin.y;
                        rect.bottom += origin.y;
                        ui.control.capture(ui.window, rect);
                    }
                } else {
                    SetCapture(ui.preview);
                    ui.gesture.press(Some(mapped), Instant::now());
                }
            }
        } else if message == WM_LBUTTONUP && ui.control.backend() == Backend::Wda {
            if let Some(action) = ui.gesture.release(map(ui, point), Instant::now()) {
                ui.control.send(control::Command::Action(action));
            }
            let _ = ReleaseCapture();
        } else if message == WM_CHAR
            && ui.control.backend() == Backend::Wda
            && ui.config.control_enabled
        {
            let unit = wparam.0 as u16;
            if (0xd800..=0xdbff).contains(&unit) {
                ui.surrogate = Some(unit);
                return;
            }
            let text = if let Some(high) = ui.surrogate.take() {
                String::from_utf16_lossy(&[high, unit])
            } else {
                String::from_utf16_lossy(&[unit])
            };
            ui.control.send(control::Command::Action(Input::Text(text)));
        }
    }
}
fn fullscreen(ui: &mut Ui) {
    ui.control.release(); // SAFETY: Save/restore the owned window's style and rectangle; video renderer remains attached unchanged.
    unsafe {
        if let Some(rect) = ui.fullscreen.take() {
            SetWindowLongPtrW(
                ui.window,
                GWL_STYLE,
                (WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE).0 as isize,
            );
            let _ = SetWindowPos(
                ui.window,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_FRAMECHANGED | SWP_NOZORDER,
            );
        } else {
            let mut rect = RECT::default();
            if GetWindowRect(ui.window, &mut rect).is_err() {
                return;
            }
            let monitor = MonitorFromWindow(ui.window, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                ui.fullscreen = Some(rect);
                SetWindowLongPtrW(
                    ui.window,
                    GWL_STYLE,
                    (WS_POPUP | WS_CLIPCHILDREN | WS_VISIBLE).0 as isize,
                );
                let r = info.rcMonitor;
                let _ = SetWindowPos(
                    ui.window,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_FRAMECHANGED | SWP_NOZORDER,
                );
            }
        }
    }
    layout(ui);
}
unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = theme::paint_message(window, message, wparam, lparam) {
        return result;
    }
    // SAFETY: Userdata points to the stable RefCell in run(). try_borrow_mut prevents Win32 reentrant aliasing.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        if message == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }
        let ptr = GetWindowLongPtrW(window, GWLP_USERDATA) as *const RefCell<Ui>;
        if !ptr.is_null()
            && let Ok(mut ui) = (&*ptr).try_borrow_mut()
        {
            if (WM_APP + WM_KEYFIRST..=WM_APP + WM_MOUSELAST).contains(&message)
                || message == WM_APP + WM_KILLFOCUS
            {
                preview_input(&mut ui, message - WM_APP, wparam, lparam);
                return LRESULT(0);
            }
            match message {
                WM_TIMER => {
                    ui.update();
                    return LRESULT(0);
                }
                WM_SIZE => {
                    ui.control.release();
                    layout(&ui);
                    return LRESULT(0);
                }
                WM_ACTIVATEAPP if wparam.0 == 0 => {
                    ui.control.release();
                    return LRESULT(0);
                }
                WM_COMMAND => {
                    let id = wparam.0 & 0xffff;
                    match id {
                        CONNECT => {
                            if ui.video.active {
                                ui.control.release();
                                ui.wanted = false;
                                ui.command(worker::Command::Disconnect);
                            } else {
                                ui.connect_device();
                            }
                        }
                        ROTATE => {
                            ui.control.release();
                            ui.rotation = (ui.rotation + 1) % 4;
                            ui.command(worker::Command::Rotate(ui.preview.0 as usize, ui.rotation));
                            layout(&ui);
                        }
                        CONTROL => {
                            let enabled = !ui.config.control_enabled;
                            ui.set_control(enabled);
                            if enabled && !ui.control.is_ready() {
                                ui.open_settings(1);
                            }
                        }
                        HOME if ui.control.capabilities().home => {
                            ui.control
                                .send(control::Command::Action(Input::Button(Button::Home)));
                        }
                        FULL => fullscreen(&mut ui),
                        SETTINGS => ui.open_settings(0),
                        ble_panel::MOVE_RIGHT
                        | ble_panel::MOVE_LEFT
                        | ble_panel::LEFT_CLICK
                        | ble_panel::TYPE_A => {
                            use imirror_input_ble::DiagnosticAction as A;
                            let action = match id {
                                ble_panel::MOVE_RIGHT => A::MoveRight,
                                ble_panel::MOVE_LEFT => A::MoveLeft,
                                ble_panel::LEFT_CLICK => A::LeftClick,
                                _ => A::TypeA,
                            };
                            ui.control.send(control::Command::BleDiagnostic(action));
                        }
                        ble_panel::SELECT_TARGET => {
                            if let Some(id) = ui.input.ble_clients.get(lparam.0 as usize) {
                                ui.control.send(control::Command::BleSelect(id.clone()));
                            }
                        }
                        ble_panel::SET_SPEED => {
                            ui.control.send(control::Command::PointerSpeed(
                                lparam.0.clamp(5, 200) as u16
                            ));
                        }
                        _ => {}
                    }
                    return LRESULT(0);
                }
                settings::EVENT => {
                    ui.settings_action(wparam.0, lparam.0);
                    return LRESULT(0);
                }
                WM_THEMECHANGED | WM_SETTINGCHANGE | THEME_CHANGED => {
                    let appearance = theme::refresh(window);
                    appearance.set_font(ui.device_label, true);
                    layout(&ui);
                    return LRESULT(0);
                }
                WM_DPICHANGED => {
                    ui.control.release();
                    theme::refresh(window);
                    let rect = &*(lparam.0 as *const RECT);
                    let _ = SetWindowPos(
                        window,
                        None,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    layout(&ui);
                    return LRESULT(0);
                }
                WM_GETMINMAXINFO => {
                    let info = &mut *(lparam.0 as *mut MINMAXINFO);
                    let appearance = theme::current(window);
                    info.ptMinTrackSize.x = appearance.px(440);
                    info.ptMinTrackSize.y = appearance.px(360);
                    return LRESULT(0);
                }
                WM_CLOSE => {
                    ui.control.release();
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
unsafe extern "system" fn preview_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Forward only scalar legacy input for capture initiation/optional WDA; BLE movement is exclusively Raw Input.
    unsafe {
        if matches!(
            message,
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_KEYDOWN | WM_CHAR | WM_KILLFOCUS
        ) {
            if message == WM_LBUTTONDOWN {
                let _ = SetFocus(Some(window));
            }
            let root = GetAncestor(window, GA_ROOT);
            let _ = PostMessageW(Some(root), WM_APP + message, wparam, lparam);
            return LRESULT(0);
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
unsafe extern "system" fn host_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Standard Win32 host behavior; no video decoding/rendering occurs here.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}
