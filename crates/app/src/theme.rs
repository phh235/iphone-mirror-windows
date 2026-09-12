//! Small native styling layer. Standard BUTTON semantics remain available to accessibility.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use windows::{
    UI::ViewManagement::{UIColorType, UISettings},
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{Dwm::*, Gdi::*},
        UI::{
            Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
            Controls::*,
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
};
thread_local! {static THEMES:RefCell<BTreeMap<usize,Rc<Theme>>>=const{RefCell::new(BTreeMap::new())};}
pub fn current(window: HWND) -> Rc<Theme> {
    if let Some(theme) = THEMES.with(|themes| themes.borrow().get(&(window.0 as usize)).cloned()) {
        return theme;
    }
    refresh(window)
}
pub fn refresh(window: HWND) -> Rc<Theme> {
    // SAFETY: The caller owns the window; absent early-create DPI falls back to 96.
    let dpi = dpi(window);
    let theme = Rc::new(Theme::new(dpi));
    if let Some(previous) = THEMES.with(|themes| themes.borrow().get(&(window.0 as usize)).cloned())
        && previous.dpi == theme.dpi
        && previous.dark == theme.dark
        && previous.high_contrast == theme.high_contrast
        && previous.background == theme.background
        && previous.text == theme.text
        && previous.accent == theme.accent
    {
        return previous;
    }
    let old = THEMES.with(|themes| themes.borrow_mut().insert(window.0 as usize, theme.clone()));
    theme.apply_window(window);
    // SAFETY: Synchronous enumeration only updates fonts on this owned window's children.
    unsafe {
        let _ = EnumChildWindows(
            Some(window),
            Some(font_child),
            LPARAM(Rc::as_ptr(&theme) as isize),
        );
        let _ = RedrawWindow(
            Some(window),
            None,
            None,
            RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_ERASE,
        );
    }
    drop(old);
    theme
}
pub fn dpi(window: HWND) -> u32 {
    // Test override affects only app-owned smoke layouts, never system settings.
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--ui-smoke-test")
        && let Some(index) = args.iter().position(|a| a == "--ui-test-dpi")
        && let Some(value) = args.get(index + 1).and_then(|v| v.parse::<u32>().ok())
        && [96, 120, 144, 168, 192].contains(&value)
    {
        return value;
    }
    // SAFETY: Read-only DPI query; early creation may return zero.
    unsafe { GetDpiForWindow(window) }.max(96)
}
unsafe extern "system" fn font_child(window: HWND, font: LPARAM) -> windows::core::BOOL {
    // SAFETY: EnumChildWindows supplies live child handles; the theme retains the font.
    unsafe {
        let theme = &*(font.0 as *const Theme);
        let mut class = [0u16; 20];
        let n = GetClassNameW(window, &mut class).max(0) as usize;
        let style = if String::from_utf16_lossy(&class[..n]).eq_ignore_ascii_case("Static") {
            GetWindowLongPtrW(window, GWLP_USERDATA)
        } else {
            0
        };
        let selected = match style {
            1 => theme.heading,
            2 => theme.small,
            3 => theme.strong,
            _ => theme.font,
        };
        SendMessageW(
            window,
            WM_SETFONT,
            Some(WPARAM(selected.0 as usize)),
            Some(LPARAM(1)),
        );
    }
    windows::core::BOOL(1)
}
pub fn forget(window: HWND) {
    THEMES.with(|themes| {
        themes.borrow_mut().remove(&(window.0 as usize));
    });
}
pub fn style_button(window: HWND) {
    // SAFETY: This subclass only handles hover; native BUTTON handles focus, keyboard and accessibility.
    unsafe {
        let _ = SetWindowSubclass(window, Some(button_proc), 1, 0);
    }
}
pub fn set_active(window: HWND, active: bool) {
    // SAFETY: GWLP_USERDATA belongs to the buttons created by iMirror.
    unsafe {
        let flags = GetWindowLongPtrW(window, GWLP_USERDATA);
        let updated = (flags & !1) | isize::from(active);
        if flags != updated {
            SetWindowLongPtrW(window, GWLP_USERDATA, updated);
            let _ = InvalidateRect(Some(window), None, false);
        }
    }
}
unsafe extern "system" fn button_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    // SAFETY: This is a registered subclass of our BUTTON control; call DefSubclassProc for native behavior.
    unsafe {
        match message {
            WM_MOUSEACTIVATE | WM_LBUTTONDOWN => {
                crate::ui_input::modality(false);
                crate::ui_input::record(window, message, lparam.0 as usize);
            }
            WM_LBUTTONUP | WM_CAPTURECHANGED | WM_SETFOCUS | WM_KILLFOCUS | BM_SETSTATE => {
                crate::ui_input::record(
                    window,
                    message,
                    if matches!(message, WM_LBUTTONUP | WM_CAPTURECHANGED) {
                        lparam.0 as usize
                    } else {
                        wparam.0
                    },
                );
            }
            WM_MOUSEMOVE => {
                if GetWindowLongPtrW(window, GWLP_USERDATA) & 2 == 0 {
                    SetWindowLongPtrW(
                        window,
                        GWLP_USERDATA,
                        GetWindowLongPtrW(window, GWLP_USERDATA) | 2,
                    );
                    let _ = InvalidateRect(Some(window), None, false);
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: window,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut track);
                }
            }
            WM_MOUSELEAVE => {
                SetWindowLongPtrW(
                    window,
                    GWLP_USERDATA,
                    GetWindowLongPtrW(window, GWLP_USERDATA) & !2,
                );
                let _ = InvalidateRect(Some(window), None, false);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(window, Some(button_proc), 1);
            }
            _ => {}
        }
        DefSubclassProc(window, message, wparam, lparam)
    }
}
pub fn paint_message(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    if message == WM_MEASUREITEM {
        // SAFETY: Windows supplies a writable MEASUREITEMSTRUCT for its own combo control.
        unsafe {
            let item = &mut *(lparam.0 as *mut MEASUREITEMSTRUCT);
            if item.CtlType == ODT_COMBOBOX {
                item.itemHeight = (24 * dpi(window) + 48) / 96;
                return Some(LRESULT(1));
            }
        }
        return None;
    }
    if message == WM_NOTIFY {
        return custom_draw(window, lparam);
    }
    if !matches!(
        message,
        WM_DRAWITEM
            | WM_ERASEBKGND
            | WM_CTLCOLORSTATIC
            | WM_CTLCOLORBTN
            | WM_CTLCOLOREDIT
            | WM_CTLCOLORLISTBOX
    ) {
        return None;
    }
    let theme = current(window);
    // SAFETY: Windows supplies message-specific live HDC/structure pointers for these synchronous paint messages.
    unsafe {
        match message {
            WM_DRAWITEM => {
                let draw = &*(lparam.0 as *const DRAWITEMSTRUCT);
                if draw.CtlType == ODT_COMBOBOX {
                    theme.draw_combo(draw);
                    return Some(LRESULT(1));
                }
                let flags = GetWindowLongPtrW(draw.hwndItem, GWLP_USERDATA);
                theme.draw_button(draw, flags & 1 != 0, flags & 2 != 0);
                Some(LRESULT(1))
            }
            WM_ERASEBKGND => {
                let mut rect = RECT::default();
                let _ = GetClientRect(window, &mut rect);
                FillRect(HDC(wparam.0 as *mut _), &rect, theme.brush);
                Some(LRESULT(1))
            }
            _ => {
                let dc = HDC(wparam.0 as *mut _);
                let mut class = [0u16; 20];
                let child = HWND(lparam.0 as *mut _);
                let n = GetClassNameW(child, &mut class).max(0) as usize;
                let secondary = message == WM_CTLCOLORSTATIC
                    && String::from_utf16_lossy(&class[..n]).eq_ignore_ascii_case("Static")
                    && GetWindowLongPtrW(child, GWLP_USERDATA) == 2;
                SetTextColor(dc, if secondary { theme.muted } else { theme.text });
                SetBkColor(dc, theme.background);
                SetBkMode(dc, TRANSPARENT);
                Some(LRESULT(theme.brush.0 as isize))
            }
        }
    }
}
fn custom_draw(parent: HWND, lparam: LPARAM) -> Option<LRESULT> {
    // SAFETY: WM_NOTIFY carries a valid NMHDR. Only NM_CUSTOMDRAW is interpreted as NMCUSTOMDRAW.
    unsafe {
        let header = &*(lparam.0 as *const NMHDR);
        if header.code != NM_CUSTOMDRAW {
            return None;
        }
        let draw = &*(lparam.0 as *const NMCUSTOMDRAW);
        let window = header.hwndFrom;
        let mut class_name = [0u16; 32];
        let class_length = GetClassNameW(window, &mut class_name).max(0) as usize;
        let class = String::from_utf16_lossy(&class_name[..class_length]);
        if class.eq_ignore_ascii_case("msctls_trackbar32") {
            if draw.dwDrawStage != CDDS_PREPAINT {
                return None;
            }
            let appearance = current(parent);
            let mut client = RECT::default();
            let mut channel = RECT::default();
            let mut thumb = RECT::default();
            let _ = GetClientRect(window, &mut client);
            SendMessageW(
                window,
                TBM_GETCHANNELRECT,
                None,
                Some(LPARAM((&mut channel as *mut RECT) as isize)),
            );
            SendMessageW(
                window,
                TBM_GETTHUMBRECT,
                None,
                Some(LPARAM((&mut thumb as *mut RECT) as isize)),
            );
            FillRect(draw.hdc, &client, appearance.brush);
            let middle = (thumb.top + thumb.bottom) / 2;
            let half = appearance.px(1).max(1);
            let old_pen = SelectObject(draw.hdc, GetStockObject(NULL_PEN));
            let old_brush = SelectObject(draw.hdc, appearance.fill(appearance.border).into());
            let _ = RoundRect(
                draw.hdc,
                channel.left,
                middle - half,
                channel.right,
                middle + half,
                half * 2,
                half * 2,
            );
            let enabled =
                windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(window).as_bool();
            SelectObject(
                draw.hdc,
                appearance
                    .fill(if enabled {
                        appearance.accent
                    } else {
                        appearance.muted
                    })
                    .into(),
            );
            let center = (thumb.left + thumb.right) / 2;
            let radius = appearance.px(5).max(2);
            let _ = Ellipse(
                draw.hdc,
                center - radius,
                middle - radius,
                center + radius,
                middle + radius,
            );
            SelectObject(draw.hdc, old_brush);
            SelectObject(draw.hdc, old_pen);
            if crate::ui_input::keyboard_focus_visible()
                && windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() == window
            {
                appearance.focus_outline(draw.hdc, &client, appearance.text);
            }
            return Some(LRESULT(CDRF_SKIPDEFAULT as isize));
        }
        if draw.dwDrawStage != CDDS_PREPAINT || !class.eq_ignore_ascii_case("Button") {
            return None;
        }
        let style = GetWindowLongPtrW(window, GWL_STYLE) as u32 & 0xf;
        if style != BS_AUTOCHECKBOX as u32 && style != BS_AUTORADIOBUTTON as u32 {
            return None;
        }
        let appearance = current(parent);
        let mut rect = RECT::default();
        let _ = GetClientRect(window, &mut rect);
        FillRect(draw.hdc, &rect, appearance.brush);
        let checked = SendMessageW(window, BM_GETCHECK, None, None).0 != 0;
        let disabled =
            !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(window).as_bool();
        let foreground = if disabled {
            appearance.muted
        } else {
            appearance.text
        };
        let side = appearance.px(18);
        let left = rect.left + appearance.px(2);
        let top = rect.top + (rect.bottom - rect.top - side) / 2;
        let fill = if checked {
            appearance.accent
        } else {
            appearance.surface
        };
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(
            PS_SOLID,
            appearance.px(1).max(1),
            if checked {
                appearance.accent
            } else {
                foreground
            },
        );
        let old_brush = SelectObject(draw.hdc, brush.into());
        let old_pen = SelectObject(draw.hdc, pen.into());
        if style == BS_AUTORADIOBUTTON as u32 {
            let _ = Ellipse(draw.hdc, left, top, left + side, top + side);
        } else {
            let _ = RoundRect(
                draw.hdc,
                left,
                top,
                left + side,
                top + side,
                appearance.px(4),
                appearance.px(4),
            );
        }
        SelectObject(draw.hdc, old_brush);
        SelectObject(draw.hdc, old_pen);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(pen.into());
        if checked {
            let mark = CreateSolidBrush(appearance.accent_text);
            let pen = CreatePen(PS_SOLID, appearance.px(2).max(1), appearance.accent_text);
            let old_brush = SelectObject(draw.hdc, mark.into());
            let old_pen = SelectObject(draw.hdc, pen.into());
            if style == BS_AUTORADIOBUTTON as u32 {
                let inset = appearance.px(5);
                let _ = Ellipse(
                    draw.hdc,
                    left + inset,
                    top + inset,
                    left + side - inset,
                    top + side - inset,
                );
            } else {
                let points = [
                    windows::Win32::Foundation::POINT {
                        x: left + appearance.px(4),
                        y: top + appearance.px(9),
                    },
                    windows::Win32::Foundation::POINT {
                        x: left + appearance.px(8),
                        y: top + appearance.px(13),
                    },
                    windows::Win32::Foundation::POINT {
                        x: left + appearance.px(14),
                        y: top + appearance.px(5),
                    },
                ];
                let _ = Polyline(draw.hdc, &points);
            }
            SelectObject(draw.hdc, old_brush);
            SelectObject(draw.hdc, old_pen);
            let _ = DeleteObject(mark.into());
            let _ = DeleteObject(pen.into());
        }
        let mut text = [0u16; 200];
        let count = GetWindowTextW(window, &mut text).max(0) as usize;
        let mut text_rect = RECT {
            left: left + side + appearance.px(8),
            ..rect
        };
        let font = SelectObject(draw.hdc, appearance.font.into());
        SetTextColor(draw.hdc, foreground);
        SetBkMode(draw.hdc, TRANSPARENT);
        DrawTextW(
            draw.hdc,
            &mut text[..count],
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        SelectObject(draw.hdc, font);
        if crate::ui_input::keyboard_focus_visible()
            && windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() == window
        {
            appearance.focus_outline(draw.hdc, &rect, foreground);
        }
        Some(LRESULT(CDRF_SKIPDEFAULT as isize))
    }
}
pub struct Theme {
    pub dark: bool,
    pub high_contrast: bool,
    pub background: COLORREF,
    pub surface: COLORREF,
    pub text: COLORREF,
    pub muted: COLORREF,
    pub border: COLORREF,
    pub hover: COLORREF,
    pub accent: COLORREF,
    pub accent_text: COLORREF,
    pub brush: HBRUSH,
    pub font: HFONT,
    pub heading: HFONT,
    pub small: HFONT,
    pub strong: HFONT,
    soft_accent: COLORREF,
    brushes: Vec<(COLORREF, HBRUSH)>,
    pens: Vec<(COLORREF, i32, HPEN)>,
    pub dpi: u32,
}
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16))
}
impl Theme {
    pub fn new(dpi: u32) -> Self {
        let settings = UISettings::new().ok();
        let foreground = settings
            .as_ref()
            .and_then(|s| s.GetColorValue(UIColorType::Foreground).ok());
        let mut dark =
            foreground.is_some_and(|c| u16::from(c.R) + u16::from(c.G) + u16::from(c.B) > 384);
        if std::env::args().any(|a| a == "--ui-smoke-test") {
            if std::env::args().any(|a| a == "--ui-test-dark") {
                dark = true;
            }
            if std::env::args().any(|a| a == "--ui-test-light") {
                dark = false;
            }
        }
        let mut contrast = HIGHCONTRASTW {
            cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
            ..Default::default()
        };
        // SAFETY: Writable initialized structure of the required size; queries system appearance only.
        let high_contrast = unsafe {
            SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                contrast.cbSize,
                Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
            )
            .is_ok()
        } && contrast.dwFlags.0 & HCF_HIGHCONTRASTON.0 != 0;
        let accent = settings
            .and_then(|s| s.GetColorValue(UIColorType::Accent).ok())
            .map(|c| rgb(c.R, c.G, c.B))
            .unwrap_or(rgb(0, 95, 184));
        let (background, surface, text, muted, border, hover, accent, accent_text) =
            if high_contrast {
                // SAFETY: System color indexes are defined constants and return scalar colors.
                unsafe {
                    (
                        COLORREF(GetSysColor(COLOR_WINDOW)),
                        COLORREF(GetSysColor(COLOR_BTNFACE)),
                        COLORREF(GetSysColor(COLOR_WINDOWTEXT)),
                        COLORREF(GetSysColor(COLOR_GRAYTEXT)),
                        COLORREF(GetSysColor(COLOR_WINDOWTEXT)),
                        COLORREF(GetSysColor(COLOR_BTNFACE)),
                        COLORREF(GetSysColor(COLOR_HIGHLIGHT)),
                        COLORREF(GetSysColor(COLOR_HIGHLIGHTTEXT)),
                    )
                }
            } else {
                let brightness = (accent.0 & 255) * 299
                    + ((accent.0 >> 8) & 255) * 587
                    + ((accent.0 >> 16) & 255) * 114;
                let accent_text = if brightness > 150_000 {
                    rgb(0, 0, 0)
                } else {
                    rgb(255, 255, 255)
                };
                if dark {
                    (
                        rgb(32, 32, 32),
                        rgb(43, 43, 43),
                        rgb(245, 245, 245),
                        rgb(180, 180, 180),
                        rgb(68, 68, 68),
                        rgb(58, 58, 58),
                        accent,
                        accent_text,
                    )
                } else {
                    (
                        rgb(249, 249, 249),
                        rgb(255, 255, 255),
                        rgb(26, 26, 26),
                        rgb(96, 96, 96),
                        rgb(221, 221, 221),
                        rgb(238, 238, 238),
                        accent,
                        accent_text,
                    )
                }
            };
        // SAFETY: GDI objects are owned by this theme and deleted after controls receive replacement fonts.
        let mix = |shift: u32| {
            let b = (background.0 >> shift) & 255;
            let a = (accent.0 >> shift) & 255;
            (b * 85 + a * 15) / 100
        };
        let soft_accent = if high_contrast {
            accent
        } else {
            COLORREF(mix(0) | (mix(8) << 8) | (mix(16) << 16))
        };
        let colors = [
            background,
            surface,
            text,
            muted,
            border,
            hover,
            accent,
            accent_text,
            soft_accent,
        ];
        let mut brushes = Vec::new();
        let mut pens = Vec::new();
        for color in colors {
            if brushes.iter().any(|(c, _)| *c == color) {
                continue;
            }
            // SAFETY: Cached GDI resources are owned by Theme and destroyed after all painting stops.
            unsafe {
                brushes.push((color, CreateSolidBrush(color)));
                for logical in [1, 2] {
                    let width = ((logical * dpi + 48) / 96).max(1) as i32;
                    pens.push((color, width, CreatePen(PS_SOLID, width, color)));
                }
            }
        }
        let brush = brushes[0].1;
        Self {
            dark,
            high_contrast,
            background,
            surface,
            text,
            muted,
            border,
            hover,
            accent,
            accent_text,
            brush,
            font: font(dpi, 14, 400),
            heading: font(dpi, 18, 600),
            small: font(dpi, 12, 400),
            strong: font(dpi, 14, 600),
            soft_accent,
            brushes,
            pens,
            dpi,
        }
    }
    pub fn px(&self, n: i32) -> i32 {
        ((i64::from(n) * i64::from(self.dpi) + 48) / 96) as i32
    }
    fn fill(&self, color: COLORREF) -> HBRUSH {
        self.brushes
            .iter()
            .find(|(c, _)| *c == color)
            .map(|(_, b)| *b)
            .unwrap_or(self.brush)
    }
    fn stroke(&self, color: COLORREF, width: i32) -> HPEN {
        self.pens
            .iter()
            .find(|(c, w, _)| *c == color && *w == width)
            .map(|(_, _, p)| *p)
            .unwrap_or(self.pens[0].2)
    }
    pub fn text_style(&self, window: HWND, style: isize) {
        // SAFETY: Caller owns this STATIC label; fonts are retained by Theme.
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, style);
            let font = match style {
                1 => self.heading,
                2 => self.small,
                3 => self.strong,
                _ => self.font,
            };
            SendMessageW(
                window,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
    pub fn apply_fonts(&self, window: HWND) {
        // SAFETY: Enumeration is synchronous; Theme stays live through every callback.
        unsafe {
            let _ = EnumChildWindows(
                Some(window),
                Some(font_child),
                LPARAM((self as *const Theme) as isize),
            );
        }
    }
    pub fn apply_window(&self, window: HWND) {
        if let Err(error) = crate::app_icon::apply(window) {
            tracing::warn!(%error, "Application icon could not be loaded");
        }
        let dark = i32::from(self.dark && !self.high_contrast);
        let corners = if self.high_contrast {
            DWMWCP_DONOTROUND
        } else {
            DWMWCP_ROUND
        };
        let backdrop = if self.high_contrast {
            DWMSBT_NONE
        } else {
            DWMSBT_MAINWINDOW
        };
        // SAFETY: DWM copies fixed-size attribute values. Unsupported APIs/attributes fall back to solid Win32 drawing.
        unsafe {
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const i32).cast(),
                4,
            );
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                (&corners as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
                4,
            );
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&backdrop as *const DWM_SYSTEMBACKDROP_TYPE).cast(),
                4,
            );
        }
    }
    pub fn set_font(&self, window: HWND, heading: bool) {
        // SAFETY: Owned font outlives the child control using it.
        unsafe {
            SendMessageW(
                window,
                WM_SETFONT,
                Some(windows::Win32::Foundation::WPARAM(if heading {
                    self.heading.0
                } else {
                    self.font.0
                }
                    as usize)),
                Some(windows::Win32::Foundation::LPARAM(1)),
            );
        }
    }
    pub fn draw_button(&self, draw: &DRAWITEMSTRUCT, active: bool, hover: bool) {
        let icon = crate::toolbar::icon(draw.hwndItem);
        // SAFETY: Userdata on an owned BUTTON contains only iMirror's visual flags.
        let captured = unsafe { GetWindowLongPtrW(draw.hwndItem, GWLP_USERDATA) } & 4 != 0;
        // SAFETY: Bit 3 marks the native Settings navigation buttons.
        let navigation = unsafe { GetWindowLongPtrW(draw.hwndItem, GWLP_USERDATA) } & 8 != 0;
        let disabled = draw.itemState.0 & ODS_DISABLED.0 != 0;
        let pressed = draw.itemState.0 & ODS_SELECTED.0 != 0;
        let focused =
            draw.itemState.0 & ODS_FOCUS.0 != 0 && crate::ui_input::keyboard_focus_visible();
        let fill = if disabled {
            self.background
        } else if active && ((!navigation && icon.is_none()) || captured) {
            self.accent
        } else if pressed {
            self.border
        } else if active {
            self.soft_accent
        } else if hover {
            self.hover
        } else if icon.is_some() || navigation {
            self.background
        } else {
            self.surface
        };
        let foreground = if disabled {
            self.muted
        } else if active && ((!navigation && icon.is_none()) || captured) {
            self.accent_text
        } else {
            self.text
        };
        // SAFETY: DRAWITEMSTRUCT supplies a valid HDC for this synchronous paint. All temporary objects are restored/deleted.
        unsafe {
            FillRect(draw.hDC, &draw.rcItem, self.brush);
            let brush = self.fill(fill);
            let border = if navigation || (icon.is_some() && !active && !hover && !pressed) {
                self.background
            } else if active {
                self.accent
            } else {
                self.border
            };
            let pen = self.stroke(border, self.px(1).max(1));
            let old_brush = SelectObject(draw.hDC, brush.into());
            let old_pen = SelectObject(draw.hDC, pen.into());
            let r = draw.rcItem;
            let radius = if self.high_contrast { 0 } else { self.px(6) };
            let _ = RoundRect(
                draw.hDC,
                r.left,
                r.top,
                r.right - 1,
                r.bottom - 1,
                radius * 2,
                radius * 2,
            );
            SelectObject(draw.hDC, old_brush);
            SelectObject(draw.hDC, old_pen);
            if navigation && active {
                let marker = RECT {
                    left: r.left + self.px(2),
                    top: r.top + self.px(10),
                    right: r.left + self.px(4),
                    bottom: r.bottom - self.px(10),
                };
                FillRect(draw.hDC, &marker, self.fill(self.accent));
            }
            if let Some(icon) = icon
                && crate::svg_icons::draw(icon, draw.hDC, r, self.px(20), foreground, fill).is_ok()
            {
                if focused {
                    self.focus_outline(draw.hDC, &r, foreground);
                }
                return;
            }
            // Keep the native button's accessible text available if Windows
            // cannot create the SVG target. No mixed-family substitute icon.
            let old_font = SelectObject(draw.hDC, self.font.into());
            SetBkMode(draw.hDC, TRANSPARENT);
            SetTextColor(draw.hDC, foreground);
            let mut text = [0u16; 160];
            let count = GetWindowTextW(draw.hwndItem, &mut text).max(0) as usize;
            let mut text_rect = r;
            DrawTextW(
                draw.hDC,
                &mut text[..count],
                &mut text_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            SelectObject(draw.hDC, old_font);
            if focused {
                self.focus_outline(draw.hDC, &r, foreground);
            }
        }
    }
    fn draw_combo(&self, draw: &DRAWITEMSTRUCT) {
        // SAFETY: The native combo owns strings. Check each item's length before
        // asking Windows to copy it into a fixed buffer; never overflow that buffer.
        unsafe {
            let selected = draw.itemState.0 & ODS_SELECTED.0 != 0;
            FillRect(
                draw.hDC,
                &draw.rcItem,
                self.fill(if selected { self.accent } else { self.surface }),
            );
            SetBkMode(draw.hDC, TRANSPARENT);
            SetTextColor(
                draw.hDC,
                if selected {
                    self.accent_text
                } else {
                    self.text
                },
            );
            let previous = SelectObject(draw.hDC, self.font.into());
            let mut text = [0u16; 256];
            let length = if draw.itemID == u32::MAX {
                -1
            } else {
                SendMessageW(
                    draw.hwndItem,
                    CB_GETLBTEXTLEN,
                    Some(WPARAM(draw.itemID as usize)),
                    None,
                )
                .0
            };
            let count = if (0..256).contains(&length) {
                SendMessageW(
                    draw.hwndItem,
                    CB_GETLBTEXT,
                    Some(WPARAM(draw.itemID as usize)),
                    Some(LPARAM(text.as_mut_ptr() as isize)),
                )
                .0
                .clamp(0, 255) as usize
            } else {
                let placeholder = "No iPhone detected";
                for (to, c) in text.iter_mut().zip(placeholder.encode_utf16()) {
                    *to = c;
                }
                placeholder.len()
            };
            let mut rect = draw.rcItem;
            rect.left += self.px(8);
            rect.right -= self.px(4);
            DrawTextW(
                draw.hDC,
                &mut text[..count],
                &mut rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            SelectObject(draw.hDC, previous);
            if crate::ui_input::keyboard_focus_visible() && draw.itemState.0 & ODS_FOCUS.0 != 0 {
                self.focus_outline(draw.hDC, &draw.rcItem, self.text);
            }
        }
    }
    fn focus_outline(&self, dc: HDC, rect: &RECT, color: COLORREF) {
        // SAFETY: The outline stays inside the existing control bounds. Restore
        // selected GDI objects before freeing the temporary solid pen.
        unsafe {
            let inset = self.px(3);
            let radius = self.px(6);
            let pen = self.stroke(color, self.px(2).max(1));
            let previous_pen = SelectObject(dc, pen.into());
            let previous_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            let _ = RoundRect(
                dc,
                rect.left + inset,
                rect.top + inset,
                rect.right - inset,
                rect.bottom - inset,
                radius * 2,
                radius * 2,
            );
            SelectObject(dc, previous_brush);
            SelectObject(dc, previous_pen);
        }
    }
}
fn font(dpi: u32, pixels: i32, weight: i32) -> HFONT {
    let mut data = LOGFONTW {
        lfHeight: -((pixels as i64 * i64::from(dpi) + 48) / 96) as i32,
        lfWeight: weight,
        lfCharSet: DEFAULT_CHARSET,
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };
    let name = "Segoe UI Variable Text".encode_utf16().collect::<Vec<_>>();
    data.lfFaceName[..name.len()].copy_from_slice(&name);
    // SAFETY: LOGFONTW is fully initialized. GDI provides font substitution on systems without Segoe UI Variable.
    unsafe {
        let mut result = CreateFontIndirectW(&data);
        if result.0.is_null() {
            data.lfFaceName = [0; 32];
            let fallback = "Segoe UI".encode_utf16().collect::<Vec<_>>();
            data.lfFaceName[..fallback.len()].copy_from_slice(&fallback);
            result = CreateFontIndirectW(&data);
        }
        result
    }
}
impl Drop for Theme {
    fn drop(&mut self) {
        // SAFETY: Owned GDI handles are not selected in any app paint DC during destruction.
        unsafe {
            let _ = DeleteObject(self.font.into());
            let _ = DeleteObject(self.heading.into());
            let _ = DeleteObject(self.small.into());
            let _ = DeleteObject(self.strong.into());
            for (_, brush) in &self.brushes {
                let _ = DeleteObject((*brush).into());
            }
            for (_, _, pen) in &self.pens {
                let _ = DeleteObject((*pen).into());
            }
        }
    }
}
