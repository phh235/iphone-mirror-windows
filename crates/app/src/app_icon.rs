//! The user's app logo, distinct from the Fluent action icons.
use std::{cell::RefCell, collections::BTreeMap};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Controls::LoadIconWithScaleDown, HiDpi::*, WindowsAndMessaging::*},
    },
    core::{PCWSTR, Result},
};
struct OwnedIcon(HICON);
impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: LoadIconWithScaleDown returns an owned icon. All app windows
        // are destroyed before this UI-thread cache is dropped at thread exit.
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}
thread_local! {
    static ICONS:RefCell<BTreeMap<(i32,i32),OwnedIcon>>=const { RefCell::new(BTreeMap::new()) };
}
pub fn apply(window: HWND) -> Result<()> {
    // SAFETY: The caller owns this live top-level window. Windows copies handle
    // values; the cache retains each icon while any app window can refer to it.
    unsafe {
        let dpi = GetDpiForWindow(window).max(96);
        for (kind, width, height) in [
            (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
            (ICON_BIG, SM_CXICON, SM_CYICON),
        ] {
            let size = (
                GetSystemMetricsForDpi(width, dpi),
                GetSystemMetricsForDpi(height, dpi),
            );
            let icon = ICONS.with_borrow_mut(|cache| -> Result<HICON> {
                if let Some(icon) = cache.get(&size) {
                    return Ok(icon.0);
                }
                let icon = LoadIconWithScaleDown(
                    Some(GetModuleHandleW(None)?.into()),
                    PCWSTR(101_usize as *const u16),
                    size.0,
                    size.1,
                )?;
                cache.insert(size, OwnedIcon(icon));
                Ok(icon)
            })?;
            SendMessageW(
                window,
                WM_SETICON,
                Some(WPARAM(kind as usize)),
                Some(LPARAM(icon.0 as isize)),
            );
        }
    }
    Ok(())
}
