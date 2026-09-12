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
    let dpi = unsafe { GetDpiForWindow(window) }.max(96);
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
            LPARAM(theme.font.0 as isize),
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
unsafe extern "system" fn font_child(window: HWND, font: LPARAM) -> windows::core::BOOL {
    // SAFETY: EnumChildWindows supplies live child handles; the theme retains the font.
    unsafe {
        SendMessageW(
            window,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
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
        SetWindowLongPtrW(window, GWLP_USERDATA, (flags & !1) | isize::from(active));
        let _ = InvalidateRect(Some(window), None, false);
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
                SetTextColor(dc, theme.text);
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
            let appearance = current(parent);
            if draw.dwDrawStage == CDDS_PREPAINT {
                return Some(LRESULT(CDRF_NOTIFYITEMDRAW as isize));
            }
            if draw.dwDrawStage == CDDS_ITEMPREPAINT {
                let r = draw.rc;
                if draw.dwItemSpec as u32 == TBCD_CHANNEL {
                    let middle = (r.top + r.bottom) / 2;
                    let half = appearance.px(2).max(1);
                    let brush = CreateSolidBrush(appearance.border);
                    let previous = SelectObject(draw.hdc, brush.into());
                    let pen = SelectObject(draw.hdc, GetStockObject(NULL_PEN));
                    let _ = RoundRect(
                        draw.hdc,
                        r.left,
                        middle - half,
                        r.right,
                        middle + half,
                        half * 2,
                        half * 2,
                    );
                    SelectObject(draw.hdc, previous);
                    SelectObject(draw.hdc, pen);
                    let _ = DeleteObject(brush.into());
                } else if draw.dwItemSpec as u32 == TBCD_THUMB {
                    let brush = CreateSolidBrush(appearance.accent);
                    let previous = SelectObject(draw.hdc, brush.into());
                    let pen = SelectObject(draw.hdc, GetStockObject(NULL_PEN));
                    let side = (r.right - r.left).min(r.bottom - r.top);
                    let y = (r.top + r.bottom - side) / 2;
                    let _ = Ellipse(draw.hdc, r.left, y, r.left + side, y + side);
                    SelectObject(draw.hdc, previous);
                    SelectObject(draw.hdc, pen);
                    let _ = DeleteObject(brush.into());
                }
                return Some(LRESULT(CDRF_SKIPDEFAULT as isize));
            }
            return None;
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
        if windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() == window {
            let _ = DrawFocusRect(draw.hdc, &text_rect);
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
        let brush = unsafe { CreateSolidBrush(background) };
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
            heading: font(dpi, 17, 600),
            dpi,
        }
    }
    pub fn px(&self, n: i32) -> i32 {
        ((i64::from(n) * i64::from(self.dpi) + 48) / 96) as i32
    }
    pub fn apply_window(&self, window: HWND) {
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
        let disabled = draw.itemState.0 & ODS_DISABLED.0 != 0;
        let pressed = draw.itemState.0 & ODS_SELECTED.0 != 0;
        let focused = draw.itemState.0 & ODS_FOCUS.0 != 0;
        let fill = if disabled {
            self.background
        } else if active {
            self.accent
        } else if pressed {
            self.border
        } else if hover {
            self.hover
        } else {
            self.surface
        };
        let foreground = if disabled {
            self.muted
        } else if active {
            self.accent_text
        } else {
            self.text
        };
        // SAFETY: DRAWITEMSTRUCT supplies a valid HDC for this synchronous paint. All temporary objects are restored/deleted.
        unsafe {
            FillRect(draw.hDC, &draw.rcItem, self.brush);
            let brush = CreateSolidBrush(fill);
            let pen = CreatePen(PS_SOLID, 1, if active { self.accent } else { self.border });
            let old_brush = SelectObject(draw.hDC, brush.into());
            let old_pen = SelectObject(draw.hDC, pen.into());
            let r = draw.rcItem;
            let radius = if self.high_contrast { 0 } else { self.px(8) };
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
            let _ = DeleteObject(brush.into());
            let _ = DeleteObject(pen.into());
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
                let inset = self.px(3);
                let focus = RECT {
                    left: r.left + inset,
                    top: r.top + inset,
                    right: r.right - inset,
                    bottom: r.bottom - inset,
                };
                let _ = DrawFocusRect(draw.hDC, &focus);
            }
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
            let _ = DeleteObject(self.brush.into());
        }
    }
}
