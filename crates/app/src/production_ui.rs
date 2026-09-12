//! Compact native window. The existing worker exclusively owns video transport/decoder/renderer.
use crate::{
    ble_panel,
    control::{self, Backend, ControlManager},
    settings_window as settings, theme, toolbar,
    worker::{self, Worker},
};
use imirror_coordinate_map::{Mapper, Point, Rect, Rotation, ScaleMode, Size};
use imirror_device::{Config, ConnectionChoice, ControlChoice, DisplayChoice};
use imirror_input_core::{Button, Gesture, Input};
use std::{
    cell::{Cell, RefCell},
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
const MORE: usize = 121;
const THEME_CHANGED: u32 = WM_APP + 71;
const EXECUTE_UI_COMMAND: u32 = WM_APP + 95;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn label(window: HWND, s: &str) {
    let _ = label_changed(window, s);
}
fn label_changed(window: HWND, s: &str) -> bool {
    // SAFETY: UI-owned handle; avoid allocating text when the existing label is unchanged.
    unsafe {
        let mut current = [0u16; 256];
        let n = GetWindowTextW(window, &mut current).max(0) as usize;
        if current[..n].iter().copied().eq(s.encode_utf16()) {
            return false;
        }
        let text = wide(s);
        let _ = SetWindowTextW(window, PCWSTR(text.as_ptr()));
    }
    true
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
    more: HWND,
    tooltips: Option<toolbar::Tooltips>,
    overflow_actions: Cell<bool>,
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
    smoke_error: Option<String>,
    started: Instant,
    appearance: Option<AppearanceWatch>,
    last_geometry: (u32, u32),
    last_home: bool,
    recent_errors: std::collections::VecDeque<String>,
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
            ui.tooltips = None;
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
    fn main_action(&mut self, id: usize) {
        // SAFETY: Main action IDs refer to controls owned for the entire window lifetime.
        if let Ok(button) = unsafe { GetDlgItem(Some(self.window), id as i32) } {
            crate::ui_input::record(
                button,
                crate::ui_input::COMMAND_EXECUTED,
                usize::from(id == CONNECT && self.video.active),
            );
        }
        match id {
            CONNECT => {
                if self.video.active {
                    self.control.release();
                    self.wanted = false;
                    self.command(worker::Command::Disconnect);
                } else {
                    self.connect_device();
                }
            }
            ROTATE => {
                self.control.release();
                self.rotation = (self.rotation + 1) % 4;
                self.command(worker::Command::Rotate(
                    self.preview.0 as usize,
                    self.rotation,
                ));
                layout(self);
            }
            CONTROL => {
                let enabled = !self.config.control_enabled;
                self.set_control(enabled);
                if enabled && !self.control.is_ready() {
                    self.open_settings(1);
                }
            }
            HOME if self.control.capabilities().home => {
                self.control
                    .send(control::Command::Action(Input::Button(Button::Home)));
            }
            FULL => fullscreen(self),
            SETTINGS => self.open_settings(0),
            _ => {}
        }
    }
    fn overflow_menu(&mut self) {
        self.control.release();
        // SAFETY: Native popup owns no borrowed strings after AppendMenuW; menu is
        // destroyed after selection. The selected action is dispatched once, never as a fake click.
        unsafe {
            let menu = match CreatePopupMenu() {
                Ok(menu) => menu,
                Err(error) => {
                    self.error(&error.to_string());
                    return;
                }
            };
            let result = (|| -> windows::core::Result<usize> {
                if self.overflow_actions.get() {
                    let text = wide(if self.video.active {
                        "Disconnect"
                    } else {
                        "Connect"
                    });
                    AppendMenuW(menu, MF_STRING, CONNECT, PCWSTR(text.as_ptr()))?;
                    AppendMenuW(
                        menu,
                        if self.video.status.state == 4 {
                            MF_STRING
                        } else {
                            MF_STRING | MF_GRAYED
                        },
                        ROTATE,
                        w!("Rotate"),
                    )?;
                }
                if self.control.capabilities().home && self.input.geometry.is_some() {
                    AppendMenuW(menu, MF_STRING, HOME, w!("Home"))?;
                }
                let mut rect = RECT::default();
                GetWindowRect(self.more, &mut rect)?;
                Ok(TrackPopupMenuEx(
                    menu,
                    (TPM_RETURNCMD | TPM_RIGHTALIGN).0,
                    rect.right,
                    rect.bottom,
                    self.window,
                    None,
                )
                .0 as usize)
            })();
            let _ = DestroyMenu(menu);
            match result {
                Ok(0) => {}
                Ok(id) => self.main_action(id),
                Err(error) => self.error(&error.to_string()),
            }
        }
    }
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
        let name_changed = label_changed(
            self.device_label,
            device.map(|d| d.name.as_str()).unwrap_or("iPhone"),
        );
        let status_changed = label_changed(
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
        let captured = self.control.mailbox.captured.load(Ordering::Acquire);
        toolbar::set_control_state(self.control_button, self.config.control_enabled, captured);
        toolbar::set_icon(
            self.connect,
            if self.video.active {
                toolbar::Icon::Disconnect
            } else {
                toolbar::Icon::Connect
            },
        );
        toolbar::set_icon(
            self.full,
            if self.fullscreen.is_some() {
                toolbar::Icon::Restore
            } else {
                toolbar::Icon::Fullscreen
            },
        );
        if let Some(tips) = &self.tooltips {
            tips.set(
                self.connect,
                if self.video.active {
                    "Disconnect"
                } else {
                    "Connect"
                },
            );
            tips.set(self.rotate, "Rotate");
            tips.set(
                self.control_button,
                if captured {
                    "Control active — Ctrl+Alt+Q to release"
                } else if self.config.control_enabled && !self.control.is_ready() {
                    "Control iPhone — waiting for connection"
                } else {
                    "Control iPhone"
                },
            );
            tips.set(
                self.full,
                if self.fullscreen.is_some() {
                    "Exit fullscreen"
                } else {
                    "Fullscreen"
                },
            );
            tips.set(self.settings_button, "Settings");
            tips.set(self.more, "More actions");
        }
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
            let _ = ShowWindow(self.home, SW_HIDE); // Home remains available through the overflow menu when supported.
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
        if name_changed || status_changed {
            layout(self);
        }
    }
    fn update(&mut self) {
        if let Ok(input) = self.control.snapshots.try_recv() {
            self.input = input;
        }
        self.input.ready = self.control.is_ready();
        for message in [
            self.input.ble_error.as_deref(),
            self.input.diagnostics_error.as_deref(),
            Some(self.video.error.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            crate::diagnostics::remember(&mut self.recent_errors, message);
        }
        let home = self.control.capabilities().home && self.input.geometry.is_some();
        if home != self.last_home {
            self.last_home = home;
            layout(self);
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
        if self.smoke && self.started.elapsed().as_millis() > 1500 {
            if let Some(panel) = &self.settings
                && let Err(error) = panel.validate_navigation()
            {
                self.smoke_error = Some(error);
            }
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
                    self.smoke_error = Some(format!("UI snapshot failed: {error}"));
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
            settings::COPY_DIAGNOSTICS => {
                self.input.ready = self.control.is_ready();
                let mut report = crate::diagnostics::report(
                    &self.input,
                    &self.video,
                    &self.control.diagnostics,
                    &self.recent_errors,
                );
                report["connection_preference"] = serde_json::json!(self.config.connection);
                report["ui_input"] = crate::ui_input::snapshot();
                if let Err(error) = crate::diagnostics::copy(self.window, &report) {
                    self.error(&format!("Could not copy diagnostics: {error}"));
                }
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
    #[cfg(any(debug_assertions, feature = "ui-input-trace"))]
    let _ui_trace = crate::ui_input::TraceGuard::start()?;
    let _mta = imirror_platform_windows::Mta::new()?;
    if let Err(error) = crate::svg_icons::initialize() {
        tracing::warn!(%error, "Native SVG icon initialization failed; retaining native button labels");
    }
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
        more: HWND::default(),
        tooltips: None,
        overflow_actions: Cell::new(false),
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
        smoke_error: None,
        started: Instant::now(),
        appearance: None,
        last_geometry: (0, 0),
        last_home: false,
        recent_errors: std::collections::VecDeque::new(),
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
            ui.more = make(MORE, "More actions", w!("BUTTON"), button, window)?;
            for child in [
                ui.connect,
                ui.settings_button,
                ui.full,
                ui.home,
                ui.rotate,
                ui.control_button,
                ui.more,
            ] {
                theme::style_button(child);
            }
            for (button, icon) in [
                (ui.connect, toolbar::Icon::Connect),
                (ui.rotate, toolbar::Icon::Rotate),
                (ui.control_button, toolbar::Icon::Control),
                (ui.full, toolbar::Icon::Fullscreen),
                (ui.settings_button, toolbar::Icon::Settings),
                (ui.more, toolbar::Icon::More),
            ] {
                toolbar::set_icon(button, icon);
            }
            ui.tooltips = Some(toolbar::Tooltips::new(window)?);
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
        appearance.apply_fonts(window);
        appearance.text_style(state.borrow().device_label, 3);
        appearance.text_style(state.borrow().status_label, 2);
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
            if let Some(index) = args.iter().position(|a| a == "--ui-test-focus")
                && let Some(mode) = args.get(index + 1)
            {
                let _ = SetFocus(Some(state.borrow().settings_button));
                crate::ui_input::modality(mode == "keyboard");
            }
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
            crate::ui_input::observe_message(&message);
            let keyboard = (WM_KEYFIRST..=WM_KEYLAST).contains(&message.message);
            let in_settings = keyboard
                && panel.is_some_and(|panel| {
                    IsWindowVisible(panel).as_bool()
                        && (focus == panel || IsChild(panel, focus).as_bool())
                        && IsDialogMessageW(panel, &message).as_bool()
                });
            let preview = state.borrow().preview;
            let in_main = keyboard
                && !in_settings
                && focus != preview
                && IsDialogMessageW(window, &message).as_bool();
            if !in_settings && !in_main {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if let Some(error) = message_error {
            return Err(error.into());
        }
    }
    if let Some(error) = state.borrow_mut().smoke_error.take() {
        return Err(error.into());
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
        let measure = |window: HWND, font: HFONT| {
            let mut text = [0u16; 256];
            let n = GetWindowTextW(window, &mut text).max(0) as usize;
            let dc = GetDC(Some(ui.window));
            if dc.0.is_null() {
                return px(80);
            }
            let old = SelectObject(dc, font.into());
            let mut size = windows::Win32::Foundation::SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &text[..n], &mut size);
            SelectObject(dc, old);
            ReleaseDC(Some(ui.window), dc);
            size.cx
        };
        let device_width = measure(ui.device_label, appearance.strong);
        let status_width = measure(ui.status_label, appearance.small);
        let home = ui.control.capabilities().home && ui.input.geometry.is_some();
        let bar = toolbar::layout(width, appearance.dpi, device_width, status_width, home);
        ui.overflow_actions.set(bar.overflow);
        let top = bar.height;
        let mut action_x = bar.actions_left;
        for (button, visible) in [
            (ui.more, bar.more),
            (ui.connect, !bar.overflow),
            (ui.rotate, !bar.overflow),
            (ui.control_button, true),
            (ui.full, true),
            (ui.settings_button, true),
        ] {
            let _ = ShowWindow(button, if visible { SW_SHOWNA } else { SW_HIDE });
            if visible {
                let _ = MoveWindow(
                    button,
                    action_x,
                    (top - bar.button) / 2,
                    bar.button,
                    bar.button,
                    true,
                );
                action_x += bar.button + bar.gap;
            }
        }
        if bar.stacked {
            let _ = MoveWindow(
                ui.device_label,
                bar.padding,
                px(5),
                bar.text_width,
                px(22),
                true,
            );
            let _ = MoveWindow(
                ui.status_label,
                bar.padding,
                px(28),
                bar.text_width,
                px(18),
                true,
            );
        } else {
            let _ = MoveWindow(
                ui.device_label,
                bar.padding,
                (top - px(22)) / 2,
                device_width,
                px(22),
                true,
            );
            let offset = device_width + px(12);
            let _ = MoveWindow(
                ui.status_label,
                bar.padding + offset,
                (top - px(18)) / 2,
                (bar.text_width - offset).max(1),
                px(18),
                true,
            );
        }
        let viewport_width = width.max(1);
        let viewport_height = (height - top).max(1);
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
        let _ = ShowWindow(ui.home, SW_HIDE);
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
            crate::ui_input::record(ui.preview, crate::ui_input::PREVIEW_FOCUS_LOST, 0);
            if message == WM_KILLFOCUS && GetFocus() == ui.preview {
                return;
            }
            ui.control.release();
            ui.gesture.cancel();
            crate::ui_input::release_capture_if_owned(ui.preview);
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
            crate::ui_input::release_capture_if_owned(ui.preview);
        } else if message == WM_CHAR
            && ui.control.backend() == Backend::Wda
            && ui.config.control_enabled
            && ui.control.is_ready()
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
        if message == WM_MOUSEACTIVATE {
            crate::ui_input::modality(false);
            return LRESULT(MA_ACTIVATE as isize);
        }
        // IsDialogMessage remains the keyboard activation implementation; custom
        // top-level windows supply the same default-button contract as a dialog.
        if message == WM_USER {
            return crate::ui_input::default_button(window);
        }
        if message == WM_USER + 1 {
            return LRESULT(1);
        }
        if message == WM_COMMAND
            && [CONNECT, ROTATE, CONTROL, HOME, FULL, SETTINGS, MORE].contains(&(wparam.0 & 0xffff))
        {
            let id = wparam.0 & 0xffff;
            let native = crate::ui_input::native_click(window, wparam, lparam);
            let fullscreen_shortcut = id == FULL && lparam.0 == 0 && wparam.0 >> 16 == 0;
            if native || fullscreen_shortcut {
                if native {
                    crate::ui_input::record(HWND(lparam.0 as *mut c_void), WM_COMMAND, wparam.0);
                }
                // Exactly one dispatch, after the native BUTTON callback unwinds.
                // This is command delivery, not a retry or synthesized click.
                let _ = PostMessageW(Some(window), EXECUTE_UI_COMMAND, wparam, lparam);
            }
            return LRESULT(0);
        }
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
                WM_COMMAND | EXECUTE_UI_COMMAND => {
                    let id = wparam.0 & 0xffff;
                    match id {
                        CONNECT | ROTATE | CONTROL | HOME | FULL | SETTINGS => ui.main_action(id),
                        MORE => ui.overflow_menu(),
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
                    appearance.text_style(ui.device_label, 3);
                    appearance.text_style(ui.status_label, 2);
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
                    info.ptMinTrackSize.x = appearance.px(240);
                    info.ptMinTrackSize.y = appearance.px(240);
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
