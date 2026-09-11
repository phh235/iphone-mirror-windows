use imirror_device::{Config, Quality};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};

fn path() -> io::Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("Local application data is unavailable"))?,
    )
    .join("iMirror")
    .join("settings.json"))
}
pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
    let path = path()?;
    match fs::read(&path) {
        Ok(bytes) if bytes.len() <= 65536 => match Config::parse(&bytes) {
            Ok(config) => Ok(config),
            Err(error @ imirror_device::ConfigError::FutureVersion) => Err(error.into()),
            Err(error) => {
                if let Ok(backup) = fs::read(path.with_extension("json.bak"))
                    && backup.len() <= 65536
                    && let Ok(config) = Config::parse(&backup)
                {
                    return Ok(config);
                }
                Err(error.into())
            }
        },
        Ok(_) => Err("Settings file exceeds 64 KiB".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error.into()),
    }
}
pub fn save(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    save_to(&path()?, config)
}
fn save_to(path: &Path, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec_pretty(config)?;
    Config::parse(&bytes)?;
    let parent = path.parent().ok_or("Settings directory is unavailable")?;
    fs::create_dir_all(parent)?;
    if let Ok(old) = fs::read(path)
        && old.len() <= 65536
        && Config::parse(&old).is_ok()
    {
        fs::write(path.with_extension("json.bak"), old)?;
    }
    let temporary = parent.join(format!("settings-{:?}.tmp", windows::core::GUID::new()?));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    use std::os::windows::ffi::OsStrExt;
    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: Both paths are NUL-terminated and live through this atomic same-volume rename.
    let result = unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}
pub fn airplay_request(quality: Quality) -> (u32, u32, u32) {
    match quality {
        Quality::Auto => (1920, 1080, 60),
        Quality::Quality => (2560, 1440, 60),
        Quality::Balanced => (1920, 1080, 30),
        Quality::LowLatency => (1280, 720, 60),
    }
}
pub fn render_request(quality: Quality) -> (u32, u32, u32) {
    match quality {
        Quality::Auto | Quality::Quality => (0, 0, 0),
        Quality::Balanced => (1920, 1080, 30),
        Quality::LowLatency => (1280, 720, 60),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_settings_replace_retains_valid_backup() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "imirror-config-test-{:?}",
            windows::core::GUID::new()?
        ));
        let path = dir.join("settings.json");
        let mut config = Config::default();
        save_to(&path, &config)?;
        config.receiver_name = "Office".into();
        save_to(&path, &config)?;
        assert_eq!(Config::parse(&fs::read(&path)?)?.receiver_name, "Office");
        assert_eq!(
            Config::parse(&fs::read(path.with_extension("json.bak"))?)?.receiver_name,
            "iMirror"
        );
        config.receiver_name.clear();
        assert!(save_to(&path, &config).is_err());
        assert_eq!(Config::parse(&fs::read(&path)?)?.receiver_name, "Office");
        fs::remove_file(&path)?;
        fs::remove_file(path.with_extension("json.bak"))?;
        fs::remove_dir(dir)?;
        Ok(())
    }
}
