//! WDA-only startup preparation. Uses the existing native helper and a privately
//! registered Apple image; never downloads images, signs apps or changes drivers.
use super::*;
use std::{
    os::windows::fs::MetadataExt,
    path::{Component, Prefix},
    process::ExitStatus,
};

const MAX_OUTPUT: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartupIssue {
    #[error("Unlock the iPhone and keep it connected. Advanced control will retry automatically.")]
    DeviceLocked,
    #[error(
        "Developer Mode is off on the iPhone. Enable it in Settings > Privacy & Security, then unlock the phone."
    )]
    DeveloperModeDisabled,
    #[error(
        "The iPhone needs USB trust or developer pairing. Unlock it and complete Trust/setup before using advanced control."
    )]
    PairingRequired,
    #[error("The registered iPhone is not connected. Reconnect its USB cable and unlock it.")]
    DeviceUnavailable,
    #[error(
        "The signed WDA app is not trusted or its signature/profile is invalid. Trust or renew the signed runner on the iPhone; this cannot be repaired by reconnecting alone."
    )]
    SigningInvalid,
    #[error(
        "The registered WDA runner is not installed. Install the signed runner and register its bundle again."
    )]
    RunnerMissing,
    #[error(
        "The developer support image is not mounted and no cached image is registered. Register the Apple developer image in WDA setup."
    )]
    ImageNotRegistered,
    #[error(
        "The registered developer image is missing, incomplete or linked to another path. Register its local Restore directory again."
    )]
    ImageCacheInvalid,
    #[error(
        "Could not verify developer-image state. Check the staged WDA helper and the iPhone USB connection."
    )]
    ImageStateUnknown,
    #[error(
        "The Apple developer image could not be mounted. Keep the iPhone unlocked, check access to Apple signing services, or register a compatible image after an iOS update."
    )]
    ImageMountFailed,
    #[error(
        "Could not verify Developer Mode. Check the trusted USB connection and the staged WDA helper."
    )]
    DeveloperModeUnknown,
}
impl StartupIssue {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::DeviceLocked => "device_locked",
            Self::DeveloperModeDisabled => "developer_mode_disabled",
            Self::PairingRequired => "pairing_required",
            Self::DeviceUnavailable => "device_unavailable",
            Self::SigningInvalid => "signing_invalid",
            Self::RunnerMissing => "runner_missing",
            Self::ImageNotRegistered => "image_not_registered",
            Self::ImageCacheInvalid => "image_cache_invalid",
            Self::ImageStateUnknown => "image_state_unknown",
            Self::ImageMountFailed => "image_mount_failed",
            Self::DeveloperModeUnknown => "developer_mode_unknown",
        }
    }
}

pub(super) struct Preparation {
    registered: bool,
    state: &'static str,
    attempts: u64,
    mounted: u64,
    command_pid: Option<u32>,
}
impl Preparation {
    pub(super) fn new(registered: bool) -> Self {
        Self {
            registered,
            state: "not_checked",
            attempts: 0,
            mounted: 0,
            command_pid: None,
        }
    }
    pub(super) fn json(&self) -> Value {
        json!({"cached_image_registered":self.registered,"state":self.state,
            "mount_attempts":self.attempts,"successful_mounts":self.mounted,
            "preparation_process_id":self.command_pid})
    }
    pub(super) fn reset_check(&mut self) {
        self.state = "not_checked";
    }
    pub(super) fn failed(&mut self) {
        self.state = "blocked";
    }
}

pub(super) fn valid_local_path(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 32767
        && !text.chars().any(char::is_control)
        && Path::new(text).is_absolute()
        && matches!(Path::new(text).components().next(), Some(Component::Prefix(p))
            if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        && !Path::new(text)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
}

fn validate_image(path: &Path) -> Result<(), StartupIssue> {
    if !path.is_dir() || !path.join("BuildManifest.plist").is_file() {
        return Err(StartupIssue::ImageCacheInvalid);
    }
    // Registration is a local cache, not a UNC path or filesystem-link import.
    for item in path.ancestors() {
        let metadata = fs::symlink_metadata(item).map_err(|_| StartupIssue::ImageCacheInvalid)?;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(StartupIssue::ImageCacheInvalid);
        }
    }
    let manifest = fs::symlink_metadata(path.join("BuildManifest.plist"))
        .map_err(|_| StartupIssue::ImageCacheInvalid)?;
    if manifest.file_attributes() & 0x400 != 0 {
        return Err(StartupIssue::ImageCacheInvalid);
    }
    Ok(())
}

/// Classify known failure text into fixed messages. Never export raw error fields,
/// which may contain phone identifiers, filesystem paths or pairing material.
pub(super) fn classify_issue(component: &str, line: &str) -> Option<StartupIssue> {
    let value: Value = serde_json::from_str(line).ok()?;
    let message = value.get("msg").and_then(Value::as_str).unwrap_or("");
    let error = value.get("error").and_then(Value::as_str).unwrap_or("");
    let text = format!("{message} {error}").to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|s| text.contains(s));
    if has(&[
        "devicelocked",
        "passwordprotected",
        "device is locked",
        "device was locked",
    ]) {
        Some(StartupIssue::DeviceLocked)
    } else if has(&[
        "developermodedisabled",
        "developer mode is disabled",
        "developer mode is not enabled",
    ]) {
        Some(StartupIssue::DeveloperModeDisabled)
    } else if has(&[
        "invalidhostid",
        "userdeniedpairing",
        "pairingdialogresponsepending",
        "missingpairrecord",
        "invalidpairrecord",
    ]) {
        Some(StartupIssue::PairingRequired)
    } else if has(&[
        "no device found",
        "could not find device",
        "device not found",
    ]) {
        Some(StartupIssue::DeviceUnavailable)
    } else if component == "runner"
        && has(&[
            "code signature",
            "profile has expired",
            "untrusted developer",
            "applicationverificationfailed",
            "provisioning profile is invalid",
            "application could not be verified",
            "valid provisioning profile for this executable was not found",
            "0xe8008015",
            "0xe8008018",
        ])
    {
        Some(StartupIssue::SigningInvalid)
    } else if component == "runner"
        && has(&[
            "applicationnotfound",
            "could not find app with bundle",
            "application is not installed",
        ])
    {
        Some(StartupIssue::RunnerMissing)
    } else if component == "image-mount"
        && has(&[
            "no such file or directory",
            "system cannot find the file",
            "could not load trust-cache",
            "could not load buildmanifest",
        ])
    {
        Some(StartupIssue::ImageCacheInvalid)
    } else {
        None
    }
}

fn developer_mode(stdout: &[u8]) -> Result<bool, StartupIssue> {
    serde_json::from_slice::<Value>(stdout)
        .ok()
        .and_then(|v| v.get("DeveloperModeEnabled").and_then(Value::as_bool))
        .ok_or(StartupIssue::DeveloperModeUnknown)
}

fn image_mounted(stderr: &[u8]) -> Result<bool, StartupIssue> {
    // go-ios v1.3.2 emits structured slog messages, not an image JSON on stdout.
    let mut missing = false;
    let mut mounted = false;
    for line in stderr.split(|b| *b == b'\n') {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        match value.get("msg").and_then(Value::as_str) {
            Some("none") => missing = true,
            Some("image signature") => {
                let signature = value.get("signature").and_then(Value::as_str).unwrap_or("");
                mounted |= signature.len() >= 2
                    && signature.len() % 2 == 0
                    && signature.bytes().all(|c| c.is_ascii_hexdigit());
            }
            _ => {}
        }
    }
    match (missing, mounted) {
        (true, false) => Ok(false),
        (false, true) => Ok(true),
        _ => Err(StartupIssue::ImageStateUnknown),
    }
}

struct Output {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
type Reader = JoinHandle<io::Result<Vec<u8>>>;
struct CapturedChild<'a> {
    child: Child,
    job: Option<Job>,
    readers: Vec<Reader>,
    shared: &'a Shared,
}
impl Drop for CapturedChild<'_> {
    fn drop(&mut self) {
        drop(self.job.take());
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
        self.shared
            .preparation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .command_pid = None;
    }
}
fn bounded_output(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_OUTPUT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_OUTPUT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Helper output exceeded limit",
        ));
    }
    Ok(bytes)
}
fn capture(
    program: &Path,
    args: &[String],
    shared: &Shared,
    timeout: Duration,
    component: &'static str,
) -> Result<Output, RuntimeError> {
    if shared.cancelled() {
        return Err(RuntimeError::Cancelled);
    }
    let system = PathBuf::from(
        std::env::var_os("SystemRoot")
            .ok_or_else(|| io::Error::other("Windows directory unavailable"))?,
    )
    .join("System32");
    let job = Job::new()?;
    let mut process = Command::new(program)
        .args(args)
        .current_dir(program.parent().ok_or(RuntimeError::MissingRuntime)?)
        .env("PATH", system)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()?;
    if let Err(error) = job.assign(&process) {
        let _ = process.kill();
        let _ = process.wait();
        return Err(error);
    }
    let mut child = CapturedChild {
        child: process,
        job: Some(job),
        readers: Vec::with_capacity(2),
        shared,
    };
    let stdout = child
        .child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Missing helper stdout"))?;
    let stderr = child
        .child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Missing helper stderr"))?;
    child.readers.push(
        thread::Builder::new()
            .name("wda-setup-stdout".into())
            .spawn(|| bounded_output(stdout))?,
    );
    child.readers.push(
        thread::Builder::new()
            .name("wda-setup-stderr".into())
            .spawn(|| bounded_output(stderr))?,
    );
    shared
        .preparation
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .command_pid = Some(child.child.id());
    let started = Instant::now();
    let status = loop {
        if shared.cancelled() {
            return Err(RuntimeError::Cancelled);
        }
        if let Some(status) = child.child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            return Err(RuntimeError::Timeout(component));
        }
        if shared.wait(Duration::from_millis(100)) {
            return Err(RuntimeError::Cancelled);
        }
    };
    // Kill any inherited pipe holders in this short-lived helper's job before joining readers.
    drop(child.job.take());
    let joined: Vec<_> = child.readers.drain(..).map(JoinHandle::join).collect();
    let mut results = Vec::with_capacity(2);
    for reader in joined {
        let bytes = reader
            .map_err(|_| io::Error::other("Helper output reader failed"))?
            .map_err(|error| {
                if error.kind() == io::ErrorKind::InvalidData {
                    RuntimeError::ExcessiveOutput(component)
                } else {
                    RuntimeError::Io(error)
                }
            })?;
        results.push(bytes);
    }
    let stderr = results
        .pop()
        .ok_or_else(|| io::Error::other("Missing stderr result"))?;
    let stdout = results
        .pop()
        .ok_or_else(|| io::Error::other("Missing stdout result"))?;
    for line in stderr.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        shared.event(component, &String::from_utf8_lossy(line));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn command(
    ios: &Path,
    args: Vec<String>,
    shared: &Shared,
    timeout: Duration,
    component: &'static str,
    fallback: StartupIssue,
) -> Result<Output, RuntimeError> {
    let output = capture(ios, &args, shared, timeout, component)?;
    if !output.status.success() {
        let issue = *shared
            .startup_issue
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        return Err(issue.unwrap_or(fallback).into());
    }
    Ok(output)
}

pub(super) fn prepare(setup: &Setup, ios: &Path, shared: &Shared) -> Result<(), RuntimeError> {
    shared.publish(false, "Checking the iPhone for advanced control.");
    shared
        .preparation
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .state = "checking";
    let target = format!("--udid={}", setup.device);
    let mode = command(
        ios,
        vec!["devmode".into(), "get".into(), target.clone()],
        shared,
        Duration::from_secs(10),
        "developer-mode",
        StartupIssue::DeveloperModeUnknown,
    )?;
    if !developer_mode(&mode.stdout)? {
        return Err(StartupIssue::DeveloperModeDisabled.into());
    }
    let probe = || {
        command(
            ios,
            vec!["image".into(), "list".into(), target.clone()],
            shared,
            Duration::from_secs(10),
            "image-state",
            StartupIssue::ImageStateUnknown,
        )
    };
    if image_mounted(&probe()?.stderr)? {
        shared
            .preparation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .state = "mounted";
        return Ok(());
    }
    shared
        .preparation
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .state = "missing";
    let image = setup
        .developer_image
        .as_deref()
        .ok_or(StartupIssue::ImageNotRegistered)?;
    validate_image(image)?;
    shared.publish(false,"Restoring the iPhone developer support image. Keep the phone unlocked; Apple verification may need internet access.");
    {
        let mut state = shared.preparation.lock().unwrap_or_else(|e| e.into_inner());
        state.state = "mounting";
        state.attempts = state.attempts.saturating_add(1);
    }
    command(
        ios,
        vec![
            "image".into(),
            "mount".into(),
            format!("--path={}", image.display()),
            target.clone(),
        ],
        shared,
        Duration::from_secs(120),
        "image-mount",
        StartupIssue::ImageMountFailed,
    )?;
    if !image_mounted(&probe()?.stderr)? {
        return Err(StartupIssue::ImageMountFailed.into());
    }
    let mut state = shared.preparation.lock().unwrap_or_else(|e| e.into_inner());
    state.state = "mounted";
    state.mounted = state.mounted.saturating_add(1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mounted_state_requires_a_successful_unambiguous_signature_report() {
        assert_eq!(image_mounted(br#"{"msg":"none"}"#), Ok(false));
        assert_eq!(
            image_mounted(br#"{"msg":"image signature","signature":"aabb0102"}"#),
            Ok(true)
        );
        for input in [
            &b""[..],
            &br#"{"msg":"image signature","signature":"invalid"}"#[..],
            &br#"{"msg":"image signature"}"#[..],
            &b"{\"msg\":\"none\"}\n{\"msg\":\"image signature\",\"signature\":\"aa\"}"[..],
            &br#"{"msg":"Failed getting image list"}"#[..],
        ] {
            assert_eq!(image_mounted(input), Err(StartupIssue::ImageStateUnknown));
        }
    }
    #[test]
    fn developer_mode_must_be_a_real_boolean() {
        assert_eq!(
            developer_mode(br#"{"DeveloperModeEnabled":true}"#),
            Ok(true)
        );
        assert_eq!(
            developer_mode(br#"{"DeveloperModeEnabled":false}"#),
            Ok(false)
        );
        assert_eq!(
            developer_mode(br#"{"DeveloperModeEnabled":"true"}"#),
            Err(StartupIssue::DeveloperModeUnknown)
        );
        assert_eq!(
            developer_mode(b"{}"),
            Err(StartupIssue::DeveloperModeUnknown)
        );
    }
    #[test]
    fn cache_paths_are_local_absolute_and_not_argument_injection() {
        for path in [
            r"C:\Users\Example User\Apple\Restore",
            r"\\?\D:\images\Restore",
        ] {
            assert!(valid_local_path(path));
        }
        for path in [
            r"\\server\share\Restore",
            r"Restore",
            r"C:\image\..\Restore",
            r"\\.\device",
            "C:\\image\n--bad",
        ] {
            assert!(!valid_local_path(path));
        }
    }
    #[test]
    fn known_errors_are_classified_without_exporting_private_error_text() {
        let raw = r#"{"msg":"Failed running WDA","error":"Code signature invalid for secret-device /private/runner","pairRecord":"private-material"}"#;
        assert_eq!(
            classify_issue("runner", raw),
            Some(StartupIssue::SigningInvalid)
        );
        assert!(!safe_message(raw).contains("secret-device"));
        assert!(!safe_message(raw).contains("private-material"));
        assert_eq!(
            classify_issue("image-mount", r#"{"msg":"failure","error":"DeviceLocked"}"#),
            Some(StartupIssue::DeviceLocked)
        );
        assert_eq!(
            classify_issue(
                "image-mount",
                r#"{"msg":"requesting signature from Apple TSS"}"#
            ),
            None
        );
        assert_eq!(
            classify_issue("tunnel", r#"{"msg":"go-ios agent is not running"}"#),
            None
        );
    }
    #[test]
    fn output_is_bounded() {
        assert_eq!(bounded_output(&b"ok"[..]).ok().as_deref(), Some(&b"ok"[..]));
        assert!(bounded_output(&vec![b'x'; MAX_OUTPUT + 1][..]).is_err());
    }
    #[test]
    fn startup_command_timeout_and_cancellation_stop_owned_children()
    -> Result<(), Box<dyn std::error::Error>> {
        let ping =
            PathBuf::from(std::env::var_os("SystemRoot").ok_or("Windows directory missing")?)
                .join("System32/ping.exe");
        let args = vec!["-n".into(), "30".into(), "127.0.0.1".into()];
        let shared = Shared::new(true);
        let start = Instant::now();
        assert!(matches!(
            capture(&ping, &args, &shared, Duration::from_millis(100), "test"),
            Err(RuntimeError::Timeout("test"))
        ));
        assert!(start.elapsed() < Duration::from_secs(3));
        let state = Arc::new(Shared::new(true));
        let worker = state.clone();
        let running =
            thread::spawn(move || capture(&ping, &args, &worker, Duration::from_secs(30), "test"));
        let deadline = Instant::now() + Duration::from_secs(3);
        while state
            .preparation
            .lock()
            .map_err(|_| "Preparation poisoned")?
            .command_pid
            .is_none()
        {
            if running.is_finished() || Instant::now() >= deadline {
                *state.stopped.lock().map_err(|_| "Stop poisoned")? = true;
                state.wake.notify_all();
                let _ = running.join();
                return Err("Test child did not reach running state".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let cancelled_at = Instant::now();
        *state.stopped.lock().map_err(|_| "Stop poisoned")? = true;
        state.wake.notify_all();
        assert!(matches!(
            running.join().map_err(|_| "Capture panicked")?,
            Err(RuntimeError::Cancelled)
        ));
        assert!(cancelled_at.elapsed() < Duration::from_secs(3));
        assert!(
            state
                .preparation
                .lock()
                .map_err(|_| "Preparation poisoned")?
                .command_pid
                .is_none()
        );
        assert!(!state.status.lock().map_err(|_| "Status poisoned")?.ready);
        Ok(())
    }
}
