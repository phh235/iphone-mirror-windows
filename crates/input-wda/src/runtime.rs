//! Optional, registered WDA runtime. Never signs/installs a phone app or changes USB drivers.
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    io::{self, Read},
    net::TcpListener,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("WDA setup is invalid. Register the installed, signed runner again.")]
    InvalidSetup,
    #[error("The optional WDA runtime is missing. Install the complete runtime folder.")]
    MissingRuntime,
    #[error("WDA local port {0} is already in use. Close the old manual WDA helpers first.")]
    PortBusy(u16),
    #[error(
        "WDA {0} stopped. Keep the iPhone connected/unlocked and check Developer Mode and signing."
    )]
    HelperStopped(&'static str),
    #[error(
        "WDA {0} did not become ready. Check the iPhone's USB trust, Developer Mode and developer support image."
    )]
    Timeout(&'static str),
    #[error("WDA runtime I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("Windows could not supervise the WDA runtime: {0}")]
    Windows(#[from] windows::core::Error),
}
struct Setup {
    device: String,
    bundle: String,
}
impl Setup {
    fn parse(bytes: &[u8]) -> Result<Self, RuntimeError> {
        if bytes.len() > 65536 {
            return Err(RuntimeError::InvalidSetup);
        }
        let value: Value = serde_json::from_slice(bytes).map_err(|_| RuntimeError::InvalidSetup)?;
        let device = value["device_id"]
            .as_str()
            .ok_or(RuntimeError::InvalidSetup)?;
        let bundle = value["runner_bundle_id"]
            .as_str()
            .ok_or(RuntimeError::InvalidSetup)?;
        if value["version"].as_u64() != Some(1)
            || !(20..=80).contains(&device.len())
            || !device.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
            || !(3..=255).contains(&bundle.len())
            || !bundle.contains('.')
            || !bundle.as_bytes()[0].is_ascii_alphabetic()
            || !bundle
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        {
            return Err(RuntimeError::InvalidSetup);
        }
        Ok(Self {
            device: device.to_owned(),
            bundle: bundle.to_owned(),
        })
    }
}
#[derive(Clone)]
pub struct Status {
    pub ready: bool,
    pub generation: u64,
    pub message: String,
}
struct Shared {
    status: Mutex<Status>,
    events: Mutex<VecDeque<String>>,
    stopped: Mutex<bool>,
    wake: Condvar,
}
impl Shared {
    fn publish(&self, ready: bool, message: &str) {
        let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if s.ready != ready {
            s.generation = s.generation.wrapping_add(1);
        }
        s.ready = ready;
        s.message = message.to_owned();
    }
    fn wait(&self, duration: Duration) -> bool {
        let stop = self.stopped.lock().unwrap_or_else(|e| e.into_inner());
        let (stop, _) = self
            .wake
            .wait_timeout_while(stop, duration, |stop| !*stop)
            .unwrap_or_else(|e| e.into_inner());
        *stop
    }
    fn cancelled(&self) -> bool {
        *self.stopped.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn event(&self, component: &str, text: &str) {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if events.len() == 16 {
            events.pop_front();
        }
        events.push_back(format!("{component}: {}", safe_message(text)));
    }
}
fn safe_message(line: &str) -> String {
    // Keep the Go logger's human message, never its UDID/bundle/pairing fields.
    let json = serde_json::from_str::<Value>(line).ok();
    let text = json
        .as_ref()
        .and_then(|v| v["msg"].as_str())
        .unwrap_or("WDA helper diagnostic received");
    text.split_whitespace()
        .take(40)
        .map(|word| {
            if word.contains('@')
                || word.contains(":\\")
                || word.bytes().filter(u8::is_ascii_hexdigit).count() >= 12
            {
                "[identifier omitted]".to_owned()
            } else {
                word.chars().take(64).collect()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub struct ManagedRuntime {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}
impl ManagedRuntime {
    pub fn configured() -> bool {
        data_dir().is_ok_and(|p| p.join("setup.json").is_file())
    }
    pub fn start() -> Result<Self, RuntimeError> {
        let data = data_dir()?;
        let mut bytes = Vec::new();
        fs::File::open(data.join("setup.json"))?
            .take(65537)
            .read_to_end(&mut bytes)?;
        let setup = Setup::parse(&bytes)?;
        let exe = std::env::current_exe()?;
        let runtime = exe
            .parent()
            .ok_or(RuntimeError::MissingRuntime)?
            .join("WDA");
        if !runtime.join("ios.exe").is_file() || !runtime.join("wda-forwarder.exe").is_file() {
            return Err(RuntimeError::MissingRuntime);
        }
        fs::create_dir_all(data.join("pairing"))?;
        let shared = Arc::new(Shared {
            status: Mutex::new(Status {
                ready: false,
                generation: 0,
                message: "Starting advanced control. Keep the iPhone unlocked.".into(),
            }),
            events: Mutex::new(VecDeque::with_capacity(16)),
            stopped: Mutex::new(false),
            wake: Condvar::new(),
        });
        let worker = shared.clone();
        let thread = thread::Builder::new()
            .name("imirror-wda-runtime".into())
            .spawn(move || supervise(setup, runtime, data, worker))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
    pub fn status(&self) -> Status {
        self.shared
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn diagnostics(&self) -> Value {
        let s = self.status();
        let events = self.shared.events.lock().unwrap_or_else(|e| e.into_inner());
        json!({"managed":true,"ready":s.ready,"generation":s.generation,"message":s.message,"events":&*events})
    }
}
impl Drop for ManagedRuntime {
    fn drop(&mut self) {
        *self
            .shared
            .stopped
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = true;
        self.shared.wake.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn data_dir() -> io::Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("Local application data is unavailable"))?,
    )
    .join("iMirror/wda"))
}
struct Job(HANDLE);
impl Job {
    fn new() -> Result<Self, RuntimeError> {
        // SAFETY: Initialized limits apply only to this unnamed owned job.
        unsafe {
            let job = Self(CreateJobObjectW(None, None)?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )?;
            Ok(job)
        }
    }
    fn assign(&self, child: &Child) -> Result<(), RuntimeError> {
        // SAFETY: The child and job own live handles; no unrelated process is assigned.
        unsafe {
            AssignProcessToJobObject(self.0, HANDLE(child.as_raw_handle()))?;
        }
        Ok(())
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: Closing this unique handle terminates only our job's children.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct Services {
    job: Option<Job>,
    children: Vec<(&'static str, Child)>,
    logs: Vec<JoinHandle<()>>,
}
struct Availability(Arc<Shared>);
impl Drop for Availability {
    fn drop(&mut self) {
        self.0.publish(false, "Advanced control runtime stopped.");
    }
}
impl Services {
    fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            job: Some(Job::new()?),
            children: Vec::with_capacity(3),
            logs: Vec::with_capacity(3),
        })
    }
    fn spawn(
        &mut self,
        name: &'static str,
        program: &Path,
        args: &[String],
        shared: Arc<Shared>,
    ) -> Result<(), RuntimeError> {
        let system = PathBuf::from(
            std::env::var_os("SystemRoot")
                .ok_or_else(|| io::Error::other("Windows directory unavailable"))?,
        )
        .join("System32");
        let mut child = Command::new(program)
            .args(args)
            .current_dir(program.parent().ok_or(RuntimeError::MissingRuntime)?)
            .env("PATH", system)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()?;
        if let Err(e) = self
            .job
            .as_ref()
            .ok_or(RuntimeError::MissingRuntime)?
            .assign(&child)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        if let Some(stderr) = child.stderr.take() {
            match thread::Builder::new()
                .name(format!("wda-{name}-log"))
                .spawn(move || drain(stderr, name, shared))
            {
                Ok(thread) => self.logs.push(thread),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.into());
                }
            }
        }
        self.children.push((name, child));
        Ok(())
    }
    fn alive(&mut self) -> Result<(), RuntimeError> {
        for (name, child) in &mut self.children {
            if child.try_wait()?.is_some() {
                return Err(RuntimeError::HelperStopped(name));
            }
        }
        Ok(())
    }
}
impl Drop for Services {
    fn drop(&mut self) {
        drop(self.job.take());
        for (_, child) in &mut self.children {
            let _ = child.wait();
        }
        for log in self.logs.drain(..) {
            let _ = log.join();
        }
    }
}
fn drain(mut reader: impl Read, name: &str, shared: Arc<Shared>) {
    let mut chunk = [0; 1024];
    let mut line = Vec::with_capacity(4096);
    let mut oversized = false;
    loop {
        let n = match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &byte in &chunk[..n] {
            if byte == b'\n' {
                if !oversized && !line.is_empty() {
                    shared.event(name, &String::from_utf8_lossy(&line));
                }
                line.clear();
                oversized = false;
            } else if line.len() < 4096 && !oversized {
                line.push(byte);
            } else {
                oversized = true;
            }
        }
    }
}
fn read_json(client: &Client, url: &str) -> Option<Value> {
    let response = client.get(url).send().ok()?.error_for_status().ok()?;
    let mut bytes = Vec::new();
    response.take(65537).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 65536 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
fn server_ready(client: &Client) -> bool {
    read_json(client, "http://127.0.0.1:8100/status")
        .is_some_and(|v| v["value"]["ready"].as_bool() == Some(true))
}
fn wait_for(
    shared: &Shared,
    services: &mut Services,
    timeout: Duration,
    component: &'static str,
    mut ready: impl FnMut() -> bool,
) -> Result<bool, RuntimeError> {
    let deadline = Instant::now() + timeout;
    while !shared.cancelled() {
        services.alive()?;
        if ready() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Err(RuntimeError::Timeout(component));
        }
        if shared.wait(Duration::from_millis(200)) {
            break;
        }
    }
    Ok(false)
}
fn run_once(
    setup: &Setup,
    runtime: &Path,
    data: &Path,
    shared: &Shared,
    owner: Arc<Shared>,
    client: &Client,
) -> Result<(), RuntimeError> {
    for port in [8100, 28100] {
        drop(TcpListener::bind(("127.0.0.1", port)).map_err(|_| RuntimeError::PortBusy(port))?);
    }
    let mut services = Services::new()?;
    // Invalidate readiness before terminating owned children on any exit path.
    let _availability = Availability(owner.clone());
    let ios = runtime.join("ios.exe");
    let common = [
        format!("--udid={}", setup.device),
        "--tunnel-info-host=127.0.0.1".into(),
        "--tunnel-info-port=28100".into(),
    ];
    let mut args = vec![
        "tunnel".into(),
        "start".into(),
        "--userspace".into(),
        format!("--pair-record-path={}", data.join("pairing").display()),
    ];
    args.extend(common.iter().cloned());
    services.spawn("tunnel", &ios, &args, owner.clone())?;
    if !wait_for(
        shared,
        &mut services,
        Duration::from_secs(15),
        "USB tunnel",
        || {
            read_json(client, "http://127.0.0.1:28100/tunnels")
                .and_then(|v| {
                    v.as_array().map(|a| {
                        a.iter()
                            .any(|d| d["udid"].as_str() == Some(setup.device.as_str()))
                    })
                })
                .unwrap_or(false)
        },
    )? {
        return Ok(());
    }
    services.spawn(
        "forwarder",
        &runtime.join("wda-forwarder.exe"),
        &[
            "--mode=forward".into(),
            format!("--device={}", setup.device),
        ],
        owner.clone(),
    )?;
    let mut args = vec![
        "runwda".into(),
        format!("--bundleid={}", setup.bundle),
        format!("--testrunnerbundleid={}", setup.bundle),
        "--xctestconfig=WebDriverAgentRunner.xctest".into(),
        "--env=USE_IP=127.0.0.1".into(),
        "--env=USE_PORT=8100".into(),
        "--env=MJPEG_SERVER_PORT=8100".into(),
    ];
    args.extend(common);
    services.spawn("runner", &ios, &args, owner)?;
    if !wait_for(
        shared,
        &mut services,
        Duration::from_secs(35),
        "signed runner",
        || server_ready(client),
    )? {
        return Ok(());
    }
    shared.publish(true, "Advanced control runtime ready.");
    let mut misses = 0;
    while !shared.wait(Duration::from_secs(2)) {
        services.alive()?;
        if server_ready(client) {
            misses = 0;
        } else {
            misses += 1;
        }
        if misses >= 2 {
            return Err(RuntimeError::Timeout("local connection"));
        }
    }
    Ok(())
}
fn supervise(setup: Setup, runtime: PathBuf, data: PathBuf, shared: Arc<Shared>) {
    let client = match Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_millis(200))
        .timeout(Duration::from_millis(600))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            shared.publish(false, "Could not initialize the local WDA health check.");
            return;
        }
    };
    let mut failures = 0usize;
    while !shared.cancelled() {
        shared.publish(
            false,
            "Starting advanced control. Keep the iPhone connected and unlocked.",
        );
        let start = Instant::now();
        let result = run_once(&setup, &runtime, &data, &shared, shared.clone(), &client);
        shared.publish(false, "Advanced control stopped.");
        if shared.cancelled() {
            break;
        }
        if let Err(error) = result {
            shared.publish(false, &error.to_string());
        }
        if start.elapsed() > Duration::from_secs(60) {
            failures = 0;
        }
        let delay = retry_delay(failures);
        failures = failures.saturating_add(1);
        if shared.wait(delay) {
            break;
        }
    }
}
fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs([1, 2, 5, 10, 30][attempt.min(4)])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shared() -> Arc<Shared> {
        Arc::new(Shared {
            status: Mutex::new(Status {
                ready: false,
                generation: 0,
                message: String::new(),
            }),
            events: Mutex::new(VecDeque::new()),
            stopped: Mutex::new(false),
            wake: Condvar::new(),
        })
    }
    #[test]
    fn cancellation_wakes_backoff_and_readiness_changes_epoch()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = shared();
        state.publish(true, "ready");
        let before = state
            .status
            .lock()
            .map_err(|_| "state poisoned")?
            .generation;
        state.publish(false, "disconnected");
        assert!(
            state
                .status
                .lock()
                .map_err(|_| "state poisoned")?
                .generation
                > before
        );
        let worker = state.clone();
        let started = Instant::now();
        let join = thread::spawn(move || worker.wait(Duration::from_secs(30)));
        *state.stopped.lock().map_err(|_| "stop poisoned")? = true;
        state.wake.notify_all();
        assert!(join.join().map_err(|_| "wait panicked")?);
        assert!(started.elapsed() < Duration::from_secs(2));
        Ok(())
    }
    #[test]
    fn owned_job_stops_its_child_and_preserves_another_process()
    -> Result<(), Box<dyn std::error::Error>> {
        struct ChildGuard(Child);
        impl Drop for ChildGuard {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let ping =
            PathBuf::from(std::env::var_os("SystemRoot").ok_or("Windows directory missing")?)
                .join("System32/ping.exe");
        let mut other = ChildGuard(
            Command::new(&ping)
                .args(["-n", "30", "127.0.0.1"])
                .creation_flags(0x08000000)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let mut owned = Services::new()?;
        owned.spawn(
            "test",
            &ping,
            &["-n".into(), "30".into(), "127.0.0.1".into()],
            shared(),
        )?;
        owned.alive()?;
        let start = Instant::now();
        drop(owned);
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(other.0.try_wait()?.is_none());
        Ok(())
    }
    #[test]
    fn setup_rejects_missing_or_injected_identifiers() {
        let valid = json!({"version":1,"device_id":"00000000-0000000000000000","runner_bundle_id":"com.example.WDARunner"});
        assert!(Setup::parse(valid.to_string().as_bytes()).is_ok());
        for (key, bad) in [
            ("device_id", "--help"),
            ("runner_bundle_id", "bad bundle\n--env=bad"),
        ] {
            let mut value = valid.clone();
            value[key] = json!(bad);
            assert!(Setup::parse(value.to_string().as_bytes()).is_err());
        }
    }
    #[test]
    fn retries_are_capped_and_logs_omit_identifiers() {
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(100), Duration::from_secs(30));
        let text =
            safe_message(r#"{"msg":"Failed device 00000000-0000000000000000","udid":"private"}"#);
        assert!(!text.contains("00000000"));
        assert!(!text.contains("private"));
    }
}
