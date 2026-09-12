//! One normal desktop instance per user. CLI diagnostics remain independent.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    path::Path,
};
use windows::{
    Win32::{
        Foundation::ERROR_SHARING_VIOLATION,
        UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, IsWindowVisible, MB_ICONINFORMATION, MessageBoxW, SW_RESTORE,
            SetForegroundWindow, ShowWindow,
        },
    },
    core::{PCWSTR, w},
};

pub struct Instance {
    _file: File,
}
impl Instance {
    pub fn acquire() -> io::Result<Option<Self>> {
        let base = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("Local application data is unavailable"))?;
        let directory = Path::new(&base).join("iMirror");
        fs::create_dir_all(&directory)?;
        Self::at_path(&directory.join("desktop-instance.lock"))
    }
    fn at_path(path: &Path) -> io::Result<Option<Self>> {
        // Windows share_mode(0) excludes another opener. The kernel releases
        // this handle on normal exit or crash; a leftover empty file is harmless.
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(path)
        {
            Ok(file) => Ok(Some(Self { _file: file })),
            Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION.0 as i32) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub fn show_existing() {
    // SAFETY: The class is owned by iMirror's normal UI. We do not synthesize
    // input; Windows decides whether the existing visible window may activate.
    unsafe {
        if let Ok(window) = FindWindowW(w!("iMirrorWindow"), None)
            && IsWindowVisible(window).as_bool()
        {
            if IsIconic(window).as_bool() {
                let _ = ShowWindow(window, SW_RESTORE);
            }
            let _ = SetForegroundWindow(window);
            return;
        }
    }
    if let Ok(config) = crate::settings::load() {
        crate::i18n::set(config.language);
    }
    let message: Vec<u16> =
        crate::i18n::tr("iMirror is still starting or closing. Please wait a moment.")
            .encode_utf16()
            .chain(Some(0))
            .collect();
    // SAFETY: Both strings are NUL terminated and retained until the dialog returns.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            w!("iMirror"),
            MB_ICONINFORMATION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_second_instance_and_reopens_after_drop() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "imirror-instance-{:?}.lock",
            windows::core::GUID::new()?
        ));
        let first = Instance::at_path(&path)?.ok_or("First lock unavailable")?;
        assert!(Instance::at_path(&path)?.is_none());
        drop(first);
        let next = Instance::at_path(&path)?.ok_or("Lock was not released")?;
        drop(next);
        fs::remove_file(path)?;
        Ok(())
    }
}
