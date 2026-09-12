//! Native modeless settings with four pages; actions are posted to the main window.
use crate::{control, theme};
use imirror_device::{Config, ConnectionChoice, ControlChoice, DisplayChoice};
use std::{cell::RefCell, ffi::c_void};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Controls::*, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
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
const APPLY: usize = 250;
const STATUS: i32 = 260;
const SPEED_LABEL: i32 = 261;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct Item {
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
}
pub struct Panel {
    window: HWND,
    context: Box<RefCell<Context>>,
    devices: Vec<String>,
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
        }));
        // SAFETY: Window is owned by this UI thread; context remains allocated until it is destroyed.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let initial_scale =
                windows::Win32::UI::HiDpi::GetDpiForWindow(owner).max(96) as f64 / 96.0;
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
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                (720.0 * initial_scale).round() as i32,
                (590.0 * initial_scale).round() as i32,
                Some(owner),
                None,
                Some(instance.into()),
                Some((&*context as *const RefCell<Context>).cast()),
            )?;
            let mut guard = CreatingWindow {
                window,
                armed: true,
            };
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
                    Some(window),
                    Some(HMENU(id as *mut c_void)),
                    Some(instance.into()),
                    None,
                )?;
                if style.0 & 0xf == BS_OWNERDRAW as u32 {
                    theme::style_button(child);
                }
                context.borrow_mut().items.push(Item {
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
            for (index, text) in ["Connection", "Control", "Display", "Advanced"]
                .iter()
                .enumerate()
            {
                add(
                    200 + index,
                    text,
                    w!("BUTTON"),
                    WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                    4,
                    12,
                    20 + index as i32 * 44,
                    136,
                    36,
                    false,
                )?;
            }
            let mut label_id = 300usize;
            let label = |id: &mut usize| {
                *id += 1;
                *id
            };
            for (page, title, subtitle) in [
                (0, "Connection", "Choose how your iPhone connects."),
                (1, "Control", "Use your mouse and keyboard with iPhone."),
                (2, "Display", "Choose how the image fits your window."),
                (
                    3,
                    "Advanced",
                    "Optional tools for setup and troubleshooting.",
                ),
            ] {
                add(
                    label(&mut label_id),
                    title,
                    w!("STATIC"),
                    WINDOW_STYLE::default(),
                    page,
                    176,
                    22,
                    476,
                    28,
                    false,
                )?;
                add(
                    label(&mut label_id),
                    subtitle,
                    w!("STATIC"),
                    WINDOW_STYLE::default(),
                    page,
                    176,
                    58,
                    476,
                    28,
                    false,
                )?;
            }
            let radio = WS_TABSTOP | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32);
            let check = WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32);
            for (i, (id, text)) in [(AUTO, "Automatic"), (USB, "USB"), (WIRELESS, "Wireless")]
                .into_iter()
                .enumerate()
            {
                add(
                    id,
                    text,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    0,
                    176,
                    100 + i as i32 * 36,
                    360,
                    32,
                    false,
                )?;
            }
            add(
                DEVICE,
                "",
                w!("COMBOBOX"),
                WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
                0,
                176,
                222,
                350,
                160,
                false,
            )?;
            add(
                REFRESH,
                "Refresh devices",
                w!("BUTTON"),
                WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                0,
                176,
                266,
                154,
                36,
                false,
            )?;
            add(
                label(&mut label_id),
                "Wireless receiver name",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                0,
                176,
                322,
                380,
                24,
                false,
            )?;
            add(
                RECEIVER,
                "iMirror",
                w!("EDIT"),
                WS_TABSTOP | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                0,
                176,
                350,
                350,
                32,
                false,
            )?;
            add(
                CONTROL,
                "Control iPhone",
                w!("BUTTON"),
                check,
                1,
                176,
                96,
                380,
                32,
                false,
            )?;
            for (i, (id, text)) in [
                (CONTROL_AUTO, "Automatic"),
                (CONTROL_BT, "Bluetooth Mouse"),
                (CONTROL_WDA, "Advanced automation (WDA)"),
            ]
            .into_iter()
            .enumerate()
            {
                add(
                    id,
                    text,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    1,
                    176,
                    144 + i as i32 * 34,
                    420,
                    30,
                    i == 2,
                )?;
            }
            add(
                SPEED_LABEL as usize,
                "Mouse sensitivity: 100%",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                1,
                176,
                254,
                420,
                28,
                false,
            )?;
            let slider = add(
                SPEED,
                "",
                w!("msctls_trackbar32"),
                WS_TABSTOP | WINDOW_STYLE(TBS_AUTOTICKS),
                1,
                176,
                286,
                410,
                36,
                false,
            )?;
            SendMessageW(slider, TBM_SETRANGEMIN, Some(WPARAM(0)), Some(LPARAM(5)));
            SendMessageW(slider, TBM_SETRANGEMAX, Some(WPARAM(1)), Some(LPARAM(200)));
            SendMessageW(slider, TBM_SETPAGESIZE, None, Some(LPARAM(20)));
            add(
                label(&mut label_id),
                "Release shortcut: Ctrl + Alt + Q",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                1,
                176,
                340,
                430,
                28,
                false,
            )?;
            add(
                STATUS as usize,
                "",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                1,
                176,
                382,
                440,
                88,
                false,
            )?;
            for (i, (id, text)) in [
                (FIT, "Fit — show the whole screen"),
                (ONE, "1:1 — one source pixel per screen pixel"),
                (FILL, "Fill — crop the edges to fill the window"),
            ]
            .into_iter()
            .enumerate()
            {
                add(
                    id,
                    text,
                    w!("BUTTON"),
                    radio
                        | if i == 0 {
                            WS_GROUP
                        } else {
                            WINDOW_STYLE::default()
                        },
                    2,
                    176,
                    104 + i as i32 * 44,
                    466,
                    36,
                    false,
                )?;
            }
            add(
                VSYNC,
                "Synchronize display",
                w!("BUTTON"),
                check,
                2,
                176,
                252,
                420,
                32,
                false,
            )?;
            add(
                ADVANCED,
                "Enable advanced features",
                w!("BUTTON"),
                check,
                3,
                176,
                102,
                430,
                32,
                false,
            )?;
            add(
                DIAGNOSTICS,
                "Diagnostics",
                w!("BUTTON"),
                WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                3,
                176,
                160,
                180,
                36,
                false,
            )?;
            add(
                WDA,
                "Connect WDA",
                w!("BUTTON"),
                WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                3,
                176,
                222,
                180,
                36,
                true,
            )?;
            add(
                label(&mut label_id),
                "WDA requires a signed iPhone runner, Developer Mode and a local connection. It is optional.",
                w!("STATIC"),
                WINDOW_STYLE::default(),
                3,
                176,
                276,
                450,
                96,
                true,
            )?;
            add(
                APPLY,
                "Done",
                w!("BUTTON"),
                WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                4,
                532,
                492,
                120,
                36,
                false,
            )?;
            let appearance = theme::refresh(window);
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
        self.context.borrow_mut().advanced = config.advanced;
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
                if let Ok(child) = GetDlgItem(Some(self.window), id as i32) {
                    SendMessageW(child, BM_SETCHECK, Some(WPARAM(usize::from(checked))), None);
                }
            }
            let ids = devices.iter().map(|d| d.id.clone()).collect::<Vec<_>>();
            if self.devices != ids {
                if let Ok(combo) = GetDlgItem(Some(self.window), DEVICE as i32) {
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
                self.devices = ids;
            }
            if let Ok(slider) = GetDlgItem(Some(self.window), SPEED as i32)
                && windows::Win32::UI::Input::KeyboardAndMouse::GetCapture() != slider
            {
                SendMessageW(
                    slider,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(input.pointer_speed_percent as isize)),
                );
            }
            let speed = wide(&format!(
                "Mouse sensitivity: {}%",
                input.pointer_speed_percent
            ));
            if let Ok(label) = GetDlgItem(Some(self.window), SPEED_LABEL) {
                let _ = SetWindowTextW(label, PCWSTR(speed.as_ptr()));
            }
            let status = if !config.control_enabled {
                "Control is off."
            } else if input.ble.mouse_ready() {
                "Ready. Click the mirrored screen to take control. Enable AssistiveTouch on iPhone if needed."
            } else if input.ble.adapter.as_ref().is_some_and(|a| !a.peripheral) {
                "This Bluetooth adapter cannot provide iPhone control. See Advanced Diagnostics."
            } else if input
                .ble
                .adapter
                .as_ref()
                .is_some_and(|a| a.radio_state != "ON")
            {
                "Turn on Windows Bluetooth to connect control."
            } else {
                "Waiting for Bluetooth. Pair this PC from iPhone Bluetooth settings, then enable AssistiveTouch."
            };
            let status = wide(status);
            if let Ok(label) = GetDlgItem(Some(self.window), STATUS) {
                let _ = SetWindowTextW(label, PCWSTR(status.as_ptr()));
            }
            if let Ok(wda) = GetDlgItem(Some(self.window), WDA as i32) {
                let _ = EnableWindow(wda, config.advanced);
            }
        }
        layout(self.window, &self.context.borrow());
    }
    pub fn receiver_name(&self) -> String {
        let mut text = [0u16; 128]; // SAFETY: Fixed writable UTF-16 buffer for owned edit control.
        unsafe {
            if let Ok(edit) = GetDlgItem(Some(self.window), RECEIVER as i32) {
                let n = GetWindowTextW(edit, &mut text).max(0) as usize;
                return String::from_utf16_lossy(&text[..n]);
            }
        }
        String::new()
    }
    pub fn set_receiver_name(&self, name: &str) {
        let name = wide(name); // SAFETY: Native edit copies the temporary NUL-terminated string.
        unsafe {
            if let Ok(edit) = GetDlgItem(Some(self.window), RECEIVER as i32) {
                let _ = SetWindowTextW(edit, PCWSTR(name.as_ptr()));
            }
        }
    }
    pub fn handle(&self) -> HWND {
        self.window
    }
}
fn layout(window: HWND, context: &Context) {
    let theme = theme::current(window); // SAFETY: All controls are owned by this UI thread and remain alive while context is borrowed.
    unsafe {
        let mut client = RECT::default();
        let _ = GetClientRect(window, &mut client);
        for item in &context.items {
            let visible = (item.page == 4 || item.page == context.page)
                && (!item.advanced || context.advanced);
            let _ = ShowWindow(item.window, if visible { SW_SHOW } else { SW_HIDE });
            let x = if item.page == 4 && item.y == 492 {
                (client.right - theme.px(140)).max(theme.px(170))
            } else {
                theme.px(item.x)
            };
            let y = if item.page == 4 && item.y == 492 {
                (client.bottom - theme.px(52)).max(theme.px(492))
            } else {
                theme.px(item.y)
            };
            let _ = MoveWindow(
                item.window,
                x,
                y,
                theme.px(item.width),
                theme.px(item.height),
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
                    if (210..243).contains(&id) && id != RECEIVER {
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
                    if let Ok(slider) = GetDlgItem(Some(window), SPEED as i32) {
                        let value = SendMessageW(slider, WM_USER, None, None).0;
                        let _ = PostMessageW(
                            Some(cell.borrow().owner),
                            EVENT,
                            WPARAM(SPEED),
                            LPARAM(value),
                        );
                    }
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
impl Drop for Panel {
    fn drop(&mut self) {
        // SAFETY: Destroy synchronously before freeing the stable context pointer.
        unsafe {
            let _ = DestroyWindow(self.window);
        }
        theme::forget(self.window);
    }
}
