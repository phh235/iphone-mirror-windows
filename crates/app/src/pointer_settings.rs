//! BLE-only preferences; independent of video/receiver settings.
use imirror_input_core::{DEFAULT_POINTER_SPEED, MAX_POINTER_SPEED, MIN_POINTER_SPEED};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};
pub fn path() -> io::Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("Local application data is unavailable"))?,
    )
    .join("iMirror/ble-input.json"))
}
fn parse(bytes: &[u8]) -> io::Result<u16> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unsupported BLE input settings version; file preserved",
        ));
    }
    let speed = value
        .get("mouse_speed_percent")
        .and_then(|v| v.as_u64())
        .filter(|v| (u64::from(MIN_POINTER_SPEED)..=u64::from(MAX_POINTER_SPEED)).contains(v))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Mouse speed must be 5-200 percent",
            )
        })?;
    Ok(speed as u16)
}
pub fn load() -> io::Result<u16> {
    let file = match fs::File::open(path()?) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(DEFAULT_POINTER_SPEED),
        Err(e) => return Err(e),
    };
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "BLE input settings exceed 4 KiB",
        ));
    }
    parse(&bytes)
}
pub fn save(percent: u16) -> io::Result<()> {
    save_to(&path()?, percent)
}
fn save_to(path: &Path, percent: u16) -> io::Result<()> {
    let bytes =
        serde_json::to_vec_pretty(&serde_json::json!({"version":1,"mouse_speed_percent":percent}))?;
    parse(&bytes)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("BLE settings directory is unavailable"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        "ble-input-{:?}.tmp",
        windows::core::GUID::new().map_err(io::Error::other)?
    ));
    let result = (|| -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: Owned NUL-terminated paths live through an atomic same-directory replacement.
        unsafe {
            MoveFileExW(
                PCWSTR(from.as_ptr()),
                PCWSTR(to.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(io::Error::other)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speed_preferences_reject_invalid_or_future_data() {
        for bytes in [
            br#"{"version":2,"mouse_speed_percent":25}"#.as_slice(),
            br#"{"version":1,"mouse_speed_percent":0}"#,
            br#"{"version":1,"mouse_speed_percent":201}"#,
            b"{}",
        ] {
            assert!(parse(bytes).is_err());
        }
        assert_eq!(
            parse(br#"{"version":1,"mouse_speed_percent":25}"#).ok(),
            Some(25)
        );
    }
    #[test]
    fn atomic_speed_save_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "imirror-pointer-test-{:?}",
            windows::core::GUID::new()?
        ));
        let file = dir.join("ble-input.json");
        save_to(&file, 25)?;
        save_to(&file, 45)?;
        assert_eq!(parse(&fs::read(&file)?)?, 45);
        assert!(save_to(&file, 0).is_err());
        assert_eq!(parse(&fs::read(&file)?)?, 45);
        fs::remove_file(file)?;
        fs::remove_dir(dir)?;
        Ok(())
    }
}
