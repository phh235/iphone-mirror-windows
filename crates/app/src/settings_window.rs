//! Native modeless settings with four pages; actions are posted to the main window.
use crate::{control, theme};
use imirror_device::{Config, ConnectionChoice, ControlChoice, DisplayChoice};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::*,
            Input::KeyboardAndMouse::EnableWindow,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};
pub const EVENT: u32 = WM_APP + 70;
pub const AUTO: usize = 210;
pub const USB: usize = 211;
pub const WIRELESS: usize = 212;
pub const DEVICE: usize = 213;
pub const REFRESH: usize = 214;
pub const RECEIVER: usize = 215;
const WIRELESS_TITLE: usize = 216;
const WIRELESS_LABEL: usize = 217;
pub const CONTROL: usize = 220;
pub const CONTROL_AUTO: usize = 221;
pub const CONTROL_BT: usize = 222;
pub const CONTROL_WDA: usize = 223;
pub const SPEED: usize = 224;
pub const FIT: usize = 230;
pub const ONE: usize = 231;
pub const FILL: usize = 232;
pub const VSYNC: usize = 233;
pub const ADVANCED: usize = 240;
pub const DIAGNOSTICS: usize = 241;
pub const WDA: usize = 242;
pub const COPY_DIAGNOSTICS: usize = 243;
const REVEAL_FOCUS: u32 = WM_APP + 91;
const CANVAS: i32 = 270;
const APPLY: usize = 250;
const STATUS: i32 = 260;
const SPEED_LABEL: i32 = 261;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct Item {
    id: usize,
    window: HWND,
    page: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    advanced: bool,
}
struct Context {
    owner: HWND,
    page: usize,
    items: Vec<Item>,
    advanced: bool,
    wireless: bool,
    pending_speed: Option<u16>,
}
pub struct Panel {
    window: HWND,
    context: Box<RefCell<Context>>,
    devices: Vec<String>,
    last_speed: Option<u16>,
    last_status: Option<&'static str>,
}
struct CreatingWindow {
    window: HWND,
    armed: bool,
}
impl Drop for CreatingWindow {
    fn drop(&mut self) {
        if self.armed {
            // SAFETY: Failed construction must revoke userdata before its context allocation is freed.
            unsafe {
                SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.window);
            }
            theme::forget(self.window);
        }
    }
}
impl Panel {
    pub fn create(owner: HWND) -> windows::core::Result<Self> {
        let context = Box::new(RefCell::new(Context {
            owner,
            page: 0,
            items: Vec::new(),
            advanced: false,
            wireless: false,
            pending_speed: None,
        }));
        // SAFETY: Window is owned by this UI thread; context remains allocated until it is destroyed.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let initial_scale = theme::dpi(owner) as f64 / 96.0;
            let mut work_area = RECT::default();
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some((&mut work_area as *mut RECT).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance.into(),
                lpszClassName: w!("iMirrorSettings"),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_win32());
            }
            let window = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("iMirrorSettings"),
                w!("iMirror Settings"),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VSCROLL | WS_HSCROLL,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                ((840.0 * initial_scale).round() as i32).min(work_area.right - work_area.left),
                ((580.0 * initial_scale).round() as i32).min(work_area.bottom - work_area.top),
                Some(owner),
                None,
                Some(instance.into()),
                Some((&*context as *const RefCell<Context>).cast()),
            )?;
            let mut guard = CreatingWindow {
                window,
                armed: true,
            };
            let canvas = CreateWindowExW(
                WS_EX_CONTROLPARENT,
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                0,
                0,
                1,
                1,
                Some(window),
                Some(HMENU(CANVAS as usize as *mut c_void)),
                Some(instance.into()),
                None,
            )?;
            if !SetWindowSubclass(canvas, Some(canvas_proc), 0x5344, window.0 as usize).as_bool() {
                return Err(windows::core::Error::from_win32());
            }
            let common = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_BAR_CLASSES,
            };
            let _ = InitCommonControlsEx(&common);
            let add = |id: usize,
                       text: &str,
                       class: PCWSTR,
                       style: WINDOW_STYLE,
                       page: usize,
                       x: i32,
                       y: i32,
                       width: i32,
                       height: i32,
                       advanced: bool|
             -> windows::core::Result<HWND> {
                let value = wide(text);
                let child = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class,
                    PCWSTR(value.as_ptr()),
                    WS_CHILD | WS_VISIBLE | style,
                    x,
                    y,
                    width,
                    height,
                    Some(if page < 4 { canvas } else { window }),
                    Some(HMENU(id as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                if style.0 & 0xf == BS_OWNERDRAW as u32 {
                    theme::style_button(child);
                }
                let _ = SetWindowSubclass(child, Some(focus_child), 0x5343, window.0 as usize);
                context.borrow_mut().items.push(Item {
                    id,
                    window: child,
                    page,
                    x,
                    y,
                    width,
                    height,
                    advanced,
                });
                Ok(child)
            };
            for (index, title) in ["Connection", "Control", "Display", "Advanced"]
                .iter()
                .enumerate()
            {
                let navigation = add(
                    200 + index,
                    title,
                    w!("BUTTON"),
                    WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                    4,
                    12,
                    16 + index as i32 * 42,
                    136,
                    38,
                    false,
                )?;
                SetWindowLongPtrW(navigation, GWLP_USERDATA, 8);
            }
            let next_label = Cell::new(300usize);
            let text = |title: &str,
                        page: usize,
                        x: i32,
                        y: i32,
                        width: i32,
                        height: i32,
                        role: isize,
                        advanced: bool|
             -> windows::core::Result<HWND> {
                let id = next_label.get() + 1;
                next_label.set(id);
                let child = add(
                    id,
                    title,
                    w!("STATIC"),
                    WINDOW_STYLE::default(),
                    page,
                    x,
                    y,
                    width,
                    height,
                    advanced,
                )?;
                SetWindowLongPtrW(child, GWLP_USERDATA, role);
                Ok(child)
            };
            for (page, title, description) in [
                (0, "Connection", "Choose how your iPhone connects."),
                (1, "Control", "Mouse and keyboard control for iPhone."),
                (2, "Display", "Choose how the image fits your window."),
                (3, "Advanced", "Optional tools and troubleshooting."),
            ] {
                text(title, page, 180, 20, 560, 28, 1, false)?;
                text(description, page, 180, 52, 560, 22, 2, false)?;
            }
            let radio = WS_TABSTOP | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32);
            let check = WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32);
            let button = WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32);
            text("Connection mode", 0, 180, 88, 520, 20, 3, false)?;
            for (i, (id, title)) in [(AUTO, "Automatic"), (USB, "USB"), (WIRELESS, "Wireless")]
                .into_iter()
                .enumerate()
            {
                add(
                    id,
                    title,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    0,
                    180,
                    114 + i as i32 * 28,
                    510,
                    26,
                    false,
                )?;
            }
            text("Device", 0, 180, 216, 520, 20, 3, false)?;
            add(
                DEVICE,
                "",
                w!("COMBOBOX"),
                WS_TABSTOP
                    | WS_VSCROLL
                    | WINDOW_STYLE((CBS_DROPDOWNLIST | CBS_OWNERDRAWFIXED | CBS_HASSTRINGS) as u32),
                0,
                180,
                242,
                390,
                160,
                false,
            )?;
            add(
                REFRESH,
                "Refresh",
                w!("BUTTON"),
                button,
                0,
                586,
                242,
                104,
                32,
                false,
            )?;
            let wireless_title = add(
                WIRELESS_TITLE,
                "Wireless",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                0,
                180,
                304,
                520,
                20,
                false,
            )?;
            SetWindowLongPtrW(wireless_title, GWLP_USERDATA, 3);
            let wireless_label = add(
                WIRELESS_LABEL,
                "Receiver name",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                0,
                180,
                328,
                520,
                20,
                false,
            )?;
            SetWindowLongPtrW(wireless_label, GWLP_USERDATA, 2);
            add(
                RECEIVER,
                "iMirror",
                w!("EDIT"),
                WS_TABSTOP | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                0,
                180,
                352,
                510,
                32,
                false,
            )?;

            text("Control", 1, 180, 88, 520, 20, 3, false)?;
            add(
                CONTROL,
                "Enable Control",
                w!("BUTTON"),
                check,
                1,
                180,
                114,
                510,
                28,
                false,
            )?;
            text("Input mode", 1, 180, 154, 520, 20, 3, false)?;
            for (i, (id, title)) in [
                (CONTROL_AUTO, "Automatic"),
                (CONTROL_BT, "Bluetooth Mouse"),
                (CONTROL_WDA, "Advanced automation (WDA)"),
            ]
            .into_iter()
            .enumerate()
            {
                add(
                    id,
                    title,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    1,
                    180,
                    181 + i as i32 * 28,
                    510,
                    26,
                    i == 2,
                )?;
            }
            text("Pointer sensitivity", 1, 180, 254, 520, 20, 3, false)?;
            let slider = add(
                SPEED,
                "Pointer sensitivity",
                w!("msctls_trackbar32"),
                WS_TABSTOP | WINDOW_STYLE(TBS_NOTICKS),
                1,
                180,
                282,
                440,
                30,
                false,
            )?;
            SendMessageW(slider, TBM_SETRANGEMIN, Some(WPARAM(0)), Some(LPARAM(5)));
            SendMessageW(slider, TBM_SETRANGEMAX, Some(WPARAM(1)), Some(LPARAM(200)));
            SendMessageW(slider, TBM_SETPAGESIZE, None, Some(LPARAM(20)));
            add(
                SPEED_LABEL as usize,
                "100%",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                1,
                636,
                285,
                60,
                24,
                false,
            )?;
            text("Release shortcut", 1, 180, 328, 520, 20, 3, false)?;
            text("Ctrl + Alt + Q", 1, 180, 351, 520, 22, 0, false)?;
            text("Status", 1, 180, 389, 520, 20, 3, false)?;
            add(
                STATUS as usize,
                "",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                1,
                180,
                413,
                530,
                48,
                false,
            )?;

            text("Scaling", 2, 180, 88, 520, 20, 3, false)?;
            for (i, (id, title, description)) in [
                (FIT, "Fit", "Show the entire iPhone screen."),
                (ONE, "1:1", "One source pixel per screen pixel."),
                (FILL, "Fill", "Fill the window; edges may be cropped."),
            ]
            .into_iter()
            .enumerate()
            {
                let y = 114 + i as i32 * 60;
                add(
                    id,
                    title,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    2,
                    180,
                    y,
                    510,
                    26,
                    false,
                )?;
                text(description, 2, 208, y + 28, 490, 20, 2, false)?;
            }
            text("Rendering", 2, 180, 310, 520, 20, 3, false)?;
            add(
                VSYNC,
                "Synchronized display",
                w!("BUTTON"),
                check,
                2,
                180,
                336,
                510,
                28,
                false,
            )?;

            text("Advanced features", 3, 180, 88, 520, 20, 3, false)?;
            add(
                ADVANCED,
                "Enable advanced features",
                w!("BUTTON"),
                check,
                3,
                180,
                114,
                510,
                28,
                false,
            )?;
            text("Troubleshooting", 3, 180, 166, 520, 20, 3, false)?;
            add(
                DIAGNOSTICS,
                "Diagnostics",
                w!("BUTTON"),
                button,
                3,
                180,
                194,
                150,
                32,
                false,
            )?;
            add(
                COPY_DIAGNOSTICS,
                "Copy Diagnostics",
                w!("BUTTON"),
                button,
                3,
                346,
                194,
                190,
                32,
                false,
            )?;
            text("WDA", 3, 180, 260, 520, 20, 3, true)?;
            add(
                WDA,
                "Connect WDA",
                w!("BUTTON"),
                button,
                3,
                180,
                288,
                150,
                32,
                true,
            )?;
            text(
                "Requires a signed iPhone runner, Developer Mode and a local connection. Optional.",
                3,
                180,
                336,
                520,
                56,
                2,
                true,
            )?;
            add(
                APPLY,
                "Done",
                w!("BUTTON"),
                button,
                4,
                700,
                492,
                100,
                32,
                false,
            )?;
            let appearance = theme::refresh(window);
            appearance.apply_fonts(window);
            for item in &context.borrow().items {
                if item.page < 4 && item.y == 22 {
                    appearance.set_font(item.window, true);
                }
            }
            layout(window, &context.borrow());
            guard.armed = false;
            Ok(Self {
                window,
                context,
                devices: Vec::new(),
                last_speed: None,
                last_status: None,
            })
        }
    }
    pub fn show(&self, page: usize) {
        self.context.borrow_mut().page = page.min(3);
        layout(self.window, &self.context.borrow()); // SAFETY: Owned modeless window, no input sent to the phone.
        unsafe {
            let _ = ShowWindow(self.window, SW_SHOW);
            let _ = SetForegroundWindow(self.window);
        }
    }
    pub fn update(
        &mut self,
        config: &Config,
        input: &control::Snapshot,
        devices: &[imirror_native_core::Device],
    ) {
        let (layout_changed, display_speed) = {
            let mut state = self.context.borrow_mut();
            let wireless = config.connection == ConnectionChoice::Wireless;
            let changed = state.advanced != config.advanced || state.wireless != wireless;
            state.advanced = config.advanced;
            state.wireless = wireless;
            if state.pending_speed == Some(input.pointer_speed_percent) {
                state.pending_speed = None;
            }
            (
                changed,
                state
                    .pending_speed
                    .unwrap_or(if input.pointer_speed_percent >= 5 {
                        input.pointer_speed_percent
                    } else {
                        100
                    }),
            )
        };
        // SAFETY: All child IDs refer to this owned settings window; state updates use native control messages.
        unsafe {
            for (id, checked) in [
                (AUTO, config.connection == ConnectionChoice::Automatic),
                (USB, config.connection == ConnectionChoice::Usb),
                (WIRELESS, config.connection == ConnectionChoice::Wireless),
                (CONTROL, config.control_enabled),
                (CONTROL_AUTO, config.control == ControlChoice::Automatic),
                (CONTROL_BT, config.control == ControlChoice::BluetoothMouse),
                (CONTROL_WDA, config.control == ControlChoice::Wda),
                (FIT, config.display == DisplayChoice::Fit),
                (ONE, config.display == DisplayChoice::OneToOne),
                (FILL, config.display == DisplayChoice::Fill),
                (VSYNC, config.vsync),
                (ADVANCED, config.advanced),
            ] {
                if let Ok(child) = child_control(self.window, id as i32)
                    && SendMessageW(child, BM_GETCHECK, None, None).0 != isize::from(checked)
                {
                    SendMessageW(child, BM_SETCHECK, Some(WPARAM(usize::from(checked))), None);
                }
            }
            if !self
                .devices
                .iter()
                .map(String::as_str)
                .eq(devices.iter().map(|d| d.id.as_str()))
            {
                if let Ok(combo) = child_control(self.window, DEVICE as i32) {
                    SendMessageW(combo, CB_RESETCONTENT, None, None);
                    for device in devices {
                        let name = wide(&device.name);
                        SendMessageW(
                            combo,
                            CB_ADDSTRING,
                            None,
                            Some(LPARAM(name.as_ptr() as isize)),
                        );
                    }
                    SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(0)), None);
                }
                self.devices = devices.iter().map(|d| d.id.clone()).collect();
            }
            if let Ok(slider) = child_control(self.window, SPEED as i32)
                && windows::Win32::UI::Input::KeyboardAndMouse::GetCapture() != slider
                && SendMessageW(slider, WM_USER, None, None).0 != display_speed as isize
            {
                SendMessageW(
                    slider,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(display_speed as isize)),
                );
            }
            if self.last_speed != Some(display_speed) {
                let speed = wide(&format!("{display_speed}%"));
                if let Ok(label) = child_control(self.window, SPEED_LABEL) {
                    let _ = SetWindowTextW(label, PCWSTR(speed.as_ptr()));
                }
                self.last_speed = Some(display_speed);
            }
            let status = control_guidance(config, input);
            if self.last_status != Some(status) {
                let value = wide(status);
                if let Ok(label) = child_control(self.window, STATUS) {
                    let _ = SetWindowTextW(label, PCWSTR(value.as_ptr()));
                }
                self.last_status = Some(status);
            }
            if let Ok(wda) = child_control(self.window, WDA as i32) {
                let _ = EnableWindow(wda, config.advanced);
            }
        }
        if layout_changed {
            layout(self.window, &self.context.borrow());
        }
    }
    pub fn receiver_name(&self) -> String {
        let mut text = [0u16; 128]; // SAFETY: Fixed writable UTF-16 buffer for owned edit control.
        unsafe {
            if let Ok(edit) = child_control(self.window, RECEIVER as i32) {
                let n = GetWindowTextW(edit, &mut text).max(0) as usize;
                return String::from_utf16_lossy(&text[..n]);
            }
        }
        String::new()
    }
    pub fn set_receiver_name(&self, name: &str) {
        let name = wide(name); // SAFETY: Native edit copies the temporary NUL-terminated string.
        unsafe {
            if let Ok(edit) = child_control(self.window, RECEIVER as i32) {
                let _ = SetWindowTextW(edit, PCWSTR(name.as_ptr()));
            }
        }
    }
    pub fn handle(&self) -> HWND {
        self.window
    }
    pub fn validate_navigation(&self) -> Result<(), String> {
        let context = self.context.borrow();
        let mut checked = 0;
        // SAFETY: Smoke test inspects and scrolls only this owned window; it never
        // dispatches an input action to the phone or changes the system DPI.
        unsafe {
            if std::env::args().any(|a| a == "--ui-test-small-window") {
                let theme = theme::current(self.window);
                SetWindowPos(
                    self.window,
                    None,
                    0,
                    0,
                    theme.px(430),
                    theme.px(320),
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .map_err(|e| e.to_string())?;
                layout(self.window, &context);
            }
            for item in &context.items {
                if !IsWindowVisible(item.window).as_bool()
                    || GetWindowLongW(item.window, GWL_STYLE) as u32 & WS_TABSTOP.0 == 0
                {
                    continue;
                }
                reveal(self.window, item.window);
                layout(self.window, &context);
                let mut rect = RECT::default();
                let mut client = RECT::default();
                let mut origin = windows::Win32::Foundation::POINT::default();
                GetWindowRect(item.window, &mut rect).map_err(|e| e.to_string())?;
                let parent = GetParent(item.window).map_err(|e| e.to_string())?;
                GetClientRect(parent, &mut client).map_err(|e| e.to_string())?;
                let _ = windows::Win32::Graphics::Gdi::ClientToScreen(parent, &mut origin);
                rect.left -= origin.x;
                rect.right -= origin.x;
                rect.top -= origin.y;
                rect.bottom -= origin.y;
                if rect.bottom <= 0
                    || rect.top >= client.bottom
                    || rect.right <= 0
                    || rect.left >= client.right
                {
                    return Err(format!(
                        "Settings control {} is unreachable",
                        GetDlgCtrlID(item.window)
                    ));
                }
                if rect.bottom - rect.top <= client.bottom - 16
                    && (rect.top < 0 || rect.bottom > client.bottom)
                {
                    return Err(
                        "Settings vertical scrolling did not reveal the complete control".into(),
                    );
                }
                checked += 1;
            }
        }
        scroll(self.window, SB_HORZ, 6, 0);
        scroll(
            self.window,
            SB_VERT,
            if std::env::args().any(|a| a == "--ui-test-scroll-end") {
                7
            } else {
                6
            },
            0,
        );
        layout(self.window, &context);
        // SAFETY: Footer is a pinned child. Verify its actual rectangle after the
        // final scroll position, not only while temporarily revealing each item.
        unsafe {
            let done = child_control(self.window, APPLY as i32).map_err(|e| e.to_string())?;
            let mut rect = RECT::default();
            let mut client = RECT::default();
            let mut origin = windows::Win32::Foundation::POINT::default();
            GetWindowRect(done, &mut rect).map_err(|e| e.to_string())?;
            GetClientRect(self.window, &mut client).map_err(|e| e.to_string())?;
            let _ = windows::Win32::Graphics::Gdi::ClientToScreen(self.window, &mut origin);
            if rect.left < origin.x
                || rect.right > origin.x + client.right
                || rect.top < origin.y
                || rect.bottom > origin.y + client.bottom
            {
                return Err("Settings Done button is clipped after scrolling".into());
            }
        }
        println!(
            "Settings navigation passed: dpi={}, controls={checked}",
            theme::dpi(self.window)
        );
        Ok(())
    }
}
fn child_control(window: HWND, id: i32) -> windows::core::Result<HWND> {
    // SAFETY: Lookup is limited to the owned settings window and its content viewport.
    unsafe {
        GetDlgItem(Some(window), id)
            .or_else(|_| GetDlgItem(Some(GetDlgItem(Some(window), CANVAS)?), id))
    }
}
unsafe extern "system" fn canvas_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    owner: usize,
) -> LRESULT {
    let parent = HWND(owner as *mut c_void);
    if message == WM_ERASEBKGND {
        // SAFETY: Erase only the viewport rectangle. Using the parent's larger
        // rectangle can paint over pinned controls when Windows supplies a print DC.
        unsafe {
            let mut rect = RECT::default();
            let _ = GetClientRect(window, &mut rect);
            windows::Win32::Graphics::Gdi::FillRect(
                windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut c_void),
                &rect,
                theme::current(parent).brush,
            );
        }
        return LRESULT(1);
    }
    if let Some(result) = theme::paint_message(parent, message, wparam, lparam) {
        return result;
    }
    // SAFETY: Forward synchronous native control notifications only to this owned
    // viewport's parent; all message buffers remain live until the call returns.
    unsafe {
        if matches!(message, WM_COMMAND | WM_HSCROLL | WM_MOUSEWHEEL) {
            return SendMessageW(parent, message, Some(wparam), Some(lparam));
        }
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(window, Some(canvas_proc), id);
        }
        DefSubclassProc(window, message, wparam, lparam)
    }
}
fn control_guidance(config: &Config, input: &control::Snapshot) -> &'static str {
    if !config.control_enabled {
        return "Control is off.";
    }
    if config.advanced && config.control == ControlChoice::Wda {
        return if input.mode == 2 && input.ready {
            "Advanced control connected."
        } else {
            "Advanced control is not ready. Start the signed runner and local connection; see Diagnostics."
        };
    }
    if input.ready && input.ble.mouse_ready() {
        "Ready. Click the iPhone screen. Ctrl+Alt+Q releases control."
    } else if input.ble.adapter.as_ref().is_some_and(|a| !a.peripheral) {
        "Bluetooth control is unavailable on this adapter. See Diagnostics."
    } else if input
        .ble
        .adapter
        .as_ref()
        .is_some_and(|a| a.radio_state != "ON")
    {
        "Turn on Windows Bluetooth."
    } else if input.ble.advertising_status == "STARTED" {
        "Pair this PC in iPhone Bluetooth settings. Enable AssistiveTouch."
    } else {
        "Starting Bluetooth control. See Diagnostics if it does not start."
    }
}
fn item_visible(item: &Item, context: &Context) -> bool {
    (item.page == 4 || item.page == context.page)
        && (!item.advanced || context.advanced)
        && (!matches!(item.id, RECEIVER | WIRELESS_TITLE | WIRELESS_LABEL) || context.wireless)
}
fn item_rect(item: &Item, context: &Context, width: i32) -> (i32, i32, i32, i32) {
    if item.page == 4 {
        return (item.x, item.y, item.width, item.height);
    }
    let mut x = item.x - 160;
    let mut y = item.y;
    let mut w = item.width;
    let mut h = item.height;
    if item.page == 1 && item.y >= 254 && context.advanced {
        y += 28;
    }
    let available = (width - 40).max(160);
    if item.width >= 480 {
        w = w.min(available);
    }
    if item.page == 0 && width < 550 {
        if item.id == DEVICE {
            w = available;
        }
        if item.id == REFRESH {
            x = 20;
            y += 42;
        }
        if item.y >= 304 {
            y += 42;
        }
    }
    if item.id == DEVICE {
        h = 32;
    }
    let slider_width = (width - 112).clamp(100, 440);
    if item.id == SPEED {
        w = slider_width;
    }
    if item.id == SPEED_LABEL as usize {
        x = 20 + slider_width + 16;
    }
    if item.page == 3 && width < 410 {
        if item.id == COPY_DIAGNOSTICS {
            x = 20;
            y += 42;
        }
        if item.y >= 260 {
            y += 42;
        }
    }
    (x, y, w, h)
}
fn layout(window: HWND, context: &Context) {
    let theme = theme::current(window); // SAFETY: All controls are owned by this UI thread and remain alive while context is borrowed.
    unsafe {
        let mut client = RECT::default();
        let _ = GetClientRect(window, &mut client);
        let Ok(canvas) = GetDlgItem(Some(window), CANVAS) else {
            return;
        };
        let sidebar = theme.px(160);
        let viewport_width = (client.right - sidebar).max(1);
        let viewport_height = (client.bottom - theme.px(64)).max(1);
        let _ = MoveWindow(canvas, sidebar, 0, viewport_width, viewport_height, true);
        let logical_width = (i64::from(viewport_width) * 96 / i64::from(theme.dpi)) as i32;
        let visible = context
            .items
            .iter()
            .filter(|i| item_visible(i, context) && i.page < 4);
        let width = visible
            .clone()
            .map(|i| {
                let r = item_rect(i, context, logical_width);
                theme.px(r.0 + r.2 + 16)
            })
            .max()
            .unwrap_or(1)
            .max(viewport_width);
        let height = visible
            .map(|i| {
                let r = item_rect(i, context, logical_width);
                theme.px(r.1 + r.3 + 16)
            })
            .max()
            .unwrap_or(1)
            .max(viewport_height);
        let mut offsets = [0; 2];
        for (index, bar, extent, page) in [
            (0, SB_HORZ, width, viewport_width),
            (1, SB_VERT, height, viewport_height),
        ] {
            let mut info = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_POS,
                ..Default::default()
            };
            let _ = GetScrollInfo(window, bar, &mut info);
            info.fMask = SIF_RANGE | SIF_PAGE | SIF_POS;
            info.nMin = 0;
            info.nMax = extent - 1;
            info.nPage = page.max(1) as u32;
            offsets[index] = SetScrollInfo(window, bar, &info, true);
        }
        for item in &context.items {
            let visible = item_visible(item, context);
            let _ = ShowWindow(item.window, if visible { SW_SHOW } else { SW_HIDE });
            let r = item_rect(item, context, logical_width);
            let x = if item.id == APPLY {
                client.right - theme.px(120)
            } else if item.page < 4 {
                theme.px(r.0) - offsets[0]
            } else {
                theme.px(item.x)
            };
            let y = if item.id == APPLY {
                client.bottom - theme.px(48)
            } else if item.page < 4 {
                theme.px(r.1) - offsets[1]
            } else {
                theme.px(item.y)
            };
            let _ = MoveWindow(
                item.window,
                x,
                y,
                theme.px(r.2),
                theme.px(if item.id == DEVICE { 160 } else { r.3 }),
                true,
            );
        }
        for i in 0..4 {
            if let Ok(button) = GetDlgItem(Some(window), 200 + i) {
                theme::set_active(button, i as usize == context.page);
            }
        }
    }
}
unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_MOUSEACTIVATE {
        crate::ui_input::modality(false);
        return LRESULT(MA_ACTIVATE as isize);
    }
    if message == WM_USER {
        return crate::ui_input::default_button(window);
    }
    if message == WM_USER + 1 {
        return LRESULT(1);
    }
    if let Some(result) = theme::paint_message(window, message, wparam, lparam) {
        return result;
    }
    // SAFETY: CREATESTRUCT carries the stable RefCell allocated by Panel; all UI mutation stays on its owning thread.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(window, GWLP_USERDATA) as *const RefCell<Context>;
        if !ptr.is_null() {
            let cell = &*ptr;
            match message {
                WM_COMMAND => {
                    let id = wparam.0 & 0xffff;
                    if (200..204).contains(&id) {
                        cell.borrow_mut().page = id - 200;
                        layout(window, &cell.borrow());
                        return LRESULT(0);
                    }
                    if id == APPLY {
                        let owner = cell.borrow().owner;
                        let _ = PostMessageW(Some(owner), EVENT, WPARAM(RECEIVER), LPARAM(0));
                        let _ = ShowWindow(window, SW_HIDE);
                        return LRESULT(0);
                    }
                    let value = if id == DEVICE {
                        SendMessageW(HWND(lparam.0 as *mut _), CB_GETCURSEL, None, None).0
                    } else {
                        SendMessageW(HWND(lparam.0 as *mut _), BM_GETCHECK, None, None).0
                    };
                    if (210..244).contains(&id) && id != RECEIVER {
                        let _ = PostMessageW(
                            Some(cell.borrow().owner),
                            EVENT,
                            WPARAM(id),
                            LPARAM(value),
                        );
                    }
                    return LRESULT(0);
                }
                WM_HSCROLL => {
                    if lparam.0 == 0 {
                        scroll(window, SB_HORZ, (wparam.0 & 0xffff) as i32, 0);
                        layout(window, &cell.borrow());
                    } else if let Ok(slider) = child_control(window, SPEED as i32) {
                        let value = SendMessageW(slider, WM_USER, None, None).0;
                        cell.borrow_mut().pending_speed = Some(value.clamp(5, 200) as u16);
                        let _ = PostMessageW(
                            Some(cell.borrow().owner),
                            EVENT,
                            WPARAM(SPEED),
                            LPARAM(value),
                        );
                    }
                    return LRESULT(0);
                }
                WM_VSCROLL => {
                    scroll(window, SB_VERT, (wparam.0 & 0xffff) as i32, 0);
                    layout(window, &cell.borrow());
                    return LRESULT(0);
                }
                WM_MOUSEWHEEL => {
                    let steps = ((wparam.0 >> 16) as u16 as i16) as i32;
                    scroll(
                        window,
                        SB_VERT,
                        -1,
                        -steps * theme::current(window).px(48) / 120,
                    );
                    layout(window, &cell.borrow());
                    return LRESULT(0);
                }
                REVEAL_FOCUS => {
                    reveal(window, HWND(lparam.0 as *mut c_void));
                    layout(window, &cell.borrow());
                    return LRESULT(0);
                }
                WM_CLOSE => {
                    let _ = ShowWindow(window, SW_HIDE);
                    return LRESULT(0);
                }
                WM_SIZE => {
                    if let Ok(context) = cell.try_borrow() {
                        layout(window, &context);
                    }
                    return LRESULT(0);
                }
                WM_GETMINMAXINFO => {
                    let info = &mut *(lparam.0 as *mut MINMAXINFO);
                    let theme = theme::current(window);
                    info.ptMinTrackSize.x = theme.px(620);
                    info.ptMinTrackSize.y = theme.px(360);
                    return LRESULT(0);
                }
                WM_DPICHANGED => {
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
                    return LRESULT(0);
                }
                WM_THEMECHANGED | WM_SETTINGCHANGE => {
                    theme::refresh(window);
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
fn scroll(window: HWND, bar: SCROLLBAR_CONSTANTS, command: i32, delta: i32) {
    // SAFETY: Window owns both native scrollbars; structure sizes and masks are initialized.
    unsafe {
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        if GetScrollInfo(window, bar, &mut info).is_err() {
            return;
        }
        let step = theme::current(window).px(32);
        info.nPos = match command {
            0 => info.nPos - step,
            1 => info.nPos + step,
            2 => info.nPos - info.nPage as i32,
            3 => info.nPos + info.nPage as i32,
            4 | 5 => info.nTrackPos,
            6 => info.nMin,
            7 => info.nMax,
            -1 => info.nPos + delta,
            _ => info.nPos,
        };
        info.fMask = SIF_POS;
        SetScrollInfo(window, bar, &info, true);
    }
}
fn reveal(window: HWND, child: HWND) {
    // SAFETY: Child was supplied by an owned subclass; validate parent before querying geometry.
    unsafe {
        let Ok(canvas) = GetDlgItem(Some(window), CANVAS) else {
            return;
        };
        if GetParent(child).ok() != Some(canvas) {
            return;
        }
        let mut bounds = RECT::default();
        let mut client = RECT::default();
        if GetWindowRect(child, &mut bounds).is_err() || GetClientRect(canvas, &mut client).is_err()
        {
            return;
        }
        let mut origin = windows::Win32::Foundation::POINT::default();
        if !windows::Win32::Graphics::Gdi::ClientToScreen(canvas, &mut origin).as_bool() {
            return;
        }
        let left = bounds.left - origin.x;
        let top = bounds.top - origin.y;
        let right = bounds.right - origin.x;
        let bottom = bounds.bottom - origin.y;
        let dx = if right - left > client.right - 16 || left < 8 {
            left - 8
        } else if right > client.right - 8 {
            right - client.right + 8
        } else {
            0
        };
        let dy = if top < 8 {
            top - 8
        } else if bottom > client.bottom - 8 {
            bottom - client.bottom + 8
        } else {
            0
        };
        scroll(window, SB_HORZ, -1, dx);
        scroll(window, SB_VERT, -1, dy);
    }
}
unsafe extern "system" fn focus_child(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    owner: usize,
) -> LRESULT {
    // SAFETY: Subclass data is our live parent HWND. Posted focus messages carry no borrowed allocation.
    unsafe {
        if message == WM_SETFOCUS {
            let _ = PostMessageW(
                Some(HWND(owner as *mut c_void)),
                REVEAL_FOCUS,
                WPARAM(0),
                LPARAM(window.0 as isize),
            );
        }
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(window, Some(focus_child), id);
        }
        DefSubclassProc(window, message, wparam, lparam)
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        // SAFETY: Destroy synchronously before freeing the stable context pointer.
        unsafe {
            let _ = DestroyWindow(self.window);
        }
        theme::forget(self.window);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_guidance_requires_started_and_wda_is_separate() {
        let mut config = Config {
            control_enabled: true,
            ..Config::default()
        };
        let mut input = control::Snapshot::default();
        for status in ["NOT_REQUESTED", "CREATED", "ABORTED", "STOPPED"] {
            input.ble.advertising_status = status.into();
            assert!(!control_guidance(&config, &input).contains("Pair"));
        }
        input.ble.advertising_status = "STARTED".into();
        assert!(control_guidance(&config, &input).contains("Pair"));
        config.advanced = true;
        config.control = ControlChoice::Wda;
        assert!(!control_guidance(&config, &input).contains("Bluetooth"));
        input.mode = 2;
        input.ready = true;
        assert_eq!(
            control_guidance(&config, &input),
            "Advanced control connected."
        );
    }
}
