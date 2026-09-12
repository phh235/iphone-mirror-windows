//! One-shot native UI capture for smoke/visual tests. Never used for iPhone video transport.
use std::{io::Write, path::Path};
use windows::{
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::*,
        Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow},
        UI::WindowsAndMessaging::{GetWindowRect, PW_RENDERFULLCONTENT},
    },
    core::Error,
};
pub fn save(window: HWND, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut rect = RECT::default();
    // SAFETY: The application captures only its own live smoke-test window.
    unsafe {
        GetWindowRect(window, &mut rect)?;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 || width > 8192 || height > 8192 {
        return Err("Invalid UI snapshot dimensions".into());
    }
    let size = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .and_then(|n| n.checked_mul(4))
        .ok_or("UI snapshot too large")?;
    // SAFETY: Compatible DC/DIB are owned locally. PrintWindow completes drawing before bytes are read.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.0.is_null() {
            return Err(Error::from_win32().into());
        }
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = DeleteDC(dc);
                return Err(error.into());
            }
        };
        let previous = SelectObject(dc, bitmap.into());
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let _ = RedrawWindow(
                Some(window),
                None,
                None,
                RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
            );
            if !PrintWindow(window, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)).as_bool() {
                return Err("Native PrintWindow failed".into());
            }
            let _ = GdiFlush();
            if pixels.is_null() {
                return Err("No UI snapshot pixels".into());
            }
            let mut file = std::fs::File::create(path)?;
            file.write_all(b"BM")?;
            file.write_all(&u32::try_from(size + 54)?.to_le_bytes())?;
            file.write_all(&[0; 4])?;
            file.write_all(&54u32.to_le_bytes())?;
            info.bmiHeader.biSizeImage = u32::try_from(size)?;
            file.write_all(std::slice::from_raw_parts(
                (&info.bmiHeader as *const BITMAPINFOHEADER).cast::<u8>(),
                40,
            ))?;
            file.write_all(std::slice::from_raw_parts(pixels.cast::<u8>(), size))?;
            Ok(())
        })();
        SelectObject(dc, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        result
    }
}
