//! Small vector icons, responsive toolbar geometry and native tooltip ownership.
use std::{cell::RefCell, collections::BTreeMap};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        Graphics::Gdi::*,
        UI::{Controls::*, WindowsAndMessaging::*},
    },
    core::{PWSTR, w},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Icon {
    Connect = 1,
    Disconnect,
    Rotate,
    Control,
    Fullscreen,
    Restore,
    Settings,
    More,
}
pub fn icon(window: HWND) -> Option<Icon> {
    // SAFETY: These flags belong only to iMirror's owner-drawn BUTTON windows.
    let value = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) };
    match (value >> 8) & 255 {
        1 => Some(Icon::Connect),
        2 => Some(Icon::Disconnect),
        3 => Some(Icon::Rotate),
        4 => Some(Icon::Control),
        5 => Some(Icon::Fullscreen),
        6 => Some(Icon::Restore),
        7 => Some(Icon::Settings),
        8 => Some(Icon::More),
        _ => None,
    }
}
pub fn set_icon(window: HWND, icon: Icon) {
    // SAFETY: Scalar flags on a UI-owned BUTTON; no layout or input state changes.
    unsafe {
        let old = GetWindowLongPtrW(window, GWLP_USERDATA);
        let new = (old & !0xff00) | ((icon as isize) << 8);
        if old != new {
            SetWindowLongPtrW(window, GWLP_USERDATA, new);
            let _ = InvalidateRect(Some(window), None, false);
        }
    }
}
pub fn set_control_state(window: HWND, enabled: bool, captured: bool) {
    // SAFETY: Bits 0/2 describe visual state, independently of the native button's capture.
    unsafe {
        let old = GetWindowLongPtrW(window, GWLP_USERDATA);
        let new = (old & !5) | isize::from(enabled) | (isize::from(captured) << 2);
        if old != new {
            SetWindowLongPtrW(window, GWLP_USERDATA, new);
            let _ = InvalidateRect(Some(window), None, false);
        }
    }
}
pub struct Layout {
    pub height: i32,
    pub button: i32,
    pub gap: i32,
    pub padding: i32,
    pub actions_left: i32,
    pub text_width: i32,
    pub stacked: bool,
    pub more: bool,
    /// Connect, Rotate, Control, Fullscreen, Settings. Hidden actions remain in More.
    pub visible: [bool; 5],
}
pub fn layout(width: i32, dpi: u32, device_text: i32, status_text: i32, home: bool) -> Layout {
    let px = |n: i32| ((i64::from(n) * i64::from(dpi) + 48) / 96) as i32;
    let button = px(34);
    let gap = px(4);
    let padding = px(10).min(((width - button) / 2).max(0));
    let wide_count = 5 + i32::from(home);
    let overflow = width - 2 * padding - (wide_count * button + (wide_count - 1) * gap) < px(86);
    let mut more = overflow || home;
    let mut visible = [!overflow, !overflow, true, true, true];
    // Never widen the video to make room for controls. Extremely narrow windows
    // progressively move actions to the same native popup, retaining the hit size.
    for index in [0, 1, 3, 4, 2] {
        let count = visible.iter().filter(|v| **v).count() as i32 + i32::from(more);
        if count * button + (count - 1).max(0) * gap + 2 * padding <= width {
            break;
        }
        visible[index] = false;
        more = true;
    }
    let count = visible.iter().filter(|v| **v).count() as i32 + i32::from(more);
    let actions_left = width - padding - (count * button + (count - 1) * gap);
    let text_width = (actions_left - padding - px(10)).max(0);
    let stacked = text_width >= px(24) && device_text + status_text + px(12) > text_width;
    Layout {
        height: px(if stacked { 54 } else { 44 }),
        button,
        gap,
        padding,
        actions_left,
        text_width,
        stacked,
        more,
        visible,
    }
}
type TipText = (String, Vec<u16>);
pub struct Tooltips {
    window: HWND,
    owner: HWND,
    texts: RefCell<BTreeMap<usize, TipText>>,
}
impl Tooltips {
    pub fn new(owner: HWND) -> windows::core::Result<Self> {
        // SAFETY: Register the standard tooltip class before creating it.
        unsafe {
            let init = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_BAR_CLASSES,
            };
            if !InitCommonControlsEx(&init).as_bool() {
                return Err(windows::core::Error::from_win32());
            }
        }
        // SAFETY: Native tooltip is owned by the main window. TTF_SUBCLASS handles normal hover timing.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                w!(""),
                WS_POPUP | WINDOW_STYLE(TTS_ALWAYSTIP | TTS_NOPREFIX),
                0,
                0,
                0,
                0,
                Some(owner),
                None,
                None,
                None,
            )?
        };
        Ok(Self {
            window,
            owner,
            texts: RefCell::new(BTreeMap::new()),
        })
    }
    pub fn set(&self, button: HWND, text: &str) {
        let text = crate::i18n::tr(text);
        let mut texts = self.texts.borrow_mut();
        let key = button.0 as usize;
        if texts.get(&key).is_some_and(|old| old.0 == text) {
            return;
        }
        let exists = texts.contains_key(&key);
        let mut value: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let info = TTTOOLINFOW {
            cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
            uFlags: TTF_IDISHWND | TTF_SUBCLASS,
            hwnd: self.owner,
            uId: key,
            lpszText: PWSTR(value.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: Tool identity is the owned child HWND. Both old/new text buffers
        // remain valid during update; the new buffer is retained until tooltip destruction.
        unsafe {
            SendMessageW(
                self.window,
                if exists {
                    TTM_UPDATETIPTEXTW
                } else {
                    TTM_ADDTOOLW
                },
                None,
                Some(LPARAM((&info as *const TTTOOLINFOW) as isize)),
            );
        }
        texts.insert(key, (text.to_owned(), value));
    }
}
impl Drop for Tooltips {
    fn drop(&mut self) {
        // SAFETY: Destroy tooltip before releasing the text buffers it references.
        unsafe {
            let _ = DestroyWindow(self.window);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolbar_has_no_bottom_reservation_or_overlapping_actions() {
        for dpi in [96, 120, 144, 168, 192] {
            for logical in [60, 90, 130, 180, 240, 300, 440, 620, 900] {
                let width = logical * dpi / 96;
                let l = layout(
                    width as i32,
                    dpi,
                    90 * dpi as i32 / 96,
                    70 * dpi as i32 / 96,
                    false,
                );
                assert!(l.text_width >= 0);
                assert!(l.actions_left >= l.padding);
                let count = l.visible.iter().filter(|v| **v).count() as i32 + i32::from(l.more);
                assert!(
                    l.actions_left + count * l.button + (count - 1) * l.gap
                        <= width as i32 - l.padding
                );
                assert!(l.height <= 54 * dpi as i32 / 96 + 1);
            }
        }
    }
}
