use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
            WindowsAndMessaging::*,
        },
    },
    core::w,
};
static CLOSED: AtomicBool = AtomicBool::new(false);
pub struct Preview(pub HWND);
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE {
        CLOSED.store(true, Ordering::Release);
        return LRESULT(0);
    }
    // SAFETY: Windows supplies the live window and its scalar message arguments.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
impl Preview {
    pub fn new() -> windows::core::Result<Self> {
        CLOSED.store(false, Ordering::Release);
        // SAFETY: Registration and lifetime of this diagnostic window stay on the current thread.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let module = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                hInstance: module.into(),
                lpszClassName: w!("iMirrorBenchmark"),
                lpfnWndProc: Some(procedure),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_win32());
            }
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("iMirrorBenchmark"),
                w!("iMirror - USB validation"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                620,
                900,
                None,
                None,
                Some(module.into()),
                None,
            )?;
            Ok(Self(hwnd))
        }
    }

    pub fn caption(&self, title: &str) {
        let text: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        // SAFETY: The window is owned by this guard; Windows copies the title.
        unsafe {
            let _ = SetWindowTextW(self.0, windows::core::PCWSTR(text.as_ptr()));
        }
    }
    pub fn pump(&self) -> bool {
        // SAFETY: This thread owns the window and its message queue.
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        !CLOSED.load(Ordering::Acquire)
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        // SAFETY: Benchmark declares this guard before the session, which therefore
        // stops its renderer before this HWND is destroyed, including error paths.
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
