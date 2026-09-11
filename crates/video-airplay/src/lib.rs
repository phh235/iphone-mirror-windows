use imirror_decoder::{DecodeError, MediaFoundationSink};
use imirror_native_core::Session;
use imirror_video_core::{EncodedVideoSink, H264Assembler};
use std::{
    io::Read,
    net::UdpSocket,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};

#[derive(Debug, thiserror::Error)]
pub enum AirPlayError {
    #[error("The AirPlay runtime is missing. Reinstall iMirror.")]
    MissingRuntime,
    #[error("Invalid AirPlay receiver name or video request")]
    InvalidConfiguration,
    #[error("AirPlay process or socket failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("AirPlay process supervision failed: {0}")]
    Windows(#[from] windows::core::Error),
}
struct Job(HANDLE);
impl Job {
    fn new() -> Result<Self, windows::core::Error> {
        // SAFETY: An unnamed job with a fully initialized limit structure is
        // owned by this guard. It contains only this receiver's child process.
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
    fn assign(&self, child: &Child) -> Result<(), windows::core::Error> {
        // SAFETY: Child owns the live process handle for the duration of this call.
        unsafe { AssignProcessToJobObject(self.0, HANDLE(child.as_raw_handle())) }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: This guard owns this unique job handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

struct StopEvent {
    handle: HANDLE,
    name: String,
}
impl StopEvent {
    fn new() -> windows::core::Result<Self> {
        let name = format!("Local\\iMirror-AirPlay-{:?}", windows::core::GUID::new()?);
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: Windows copies this unique name; the returned event is owned
        // by this guard and only the receiver child is given its name.
        let handle = unsafe {
            windows::Win32::System::Threading::CreateEventW(
                None,
                true,
                false,
                windows::core::PCWSTR(wide.as_ptr()),
            )?
        };
        Ok(Self { handle, name })
    }
    fn signal(&self) {
        // SAFETY: This event handle is live and owned by this guard.
        unsafe {
            let _ = windows::Win32::System::Threading::SetEvent(self.handle);
        }
    }
}
impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: This guard owns exactly one event handle.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub struct Receiver {
    child: Child,
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    _job: Option<Job>,
    stop_event: Option<StopEvent>,
    disconnect_event: Option<StopEvent>,
}
impl Receiver {
    pub fn start(
        session: Arc<Session>,
        runtime: &Path,
        name: &str,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Self, AirPlayError> {
        if name.is_empty()
            || name.chars().count() > 63
            || name.chars().any(char::is_control)
            || width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || !matches!(fps, 30 | 60)
        {
            return Err(AirPlayError::InvalidConfiguration);
        }
        let exe = runtime.join("UxPlay.exe");
        if !exe.is_file() {
            return Err(AirPlayError::MissingRuntime);
        }
        let video = UdpSocket::bind(("127.0.0.1", 0))?;
        socket2::SockRef::from(&video).set_recv_buffer_size(4 * 1024 * 1024)?;
        video.set_read_timeout(Some(Duration::from_millis(100)))?;
        let video_pipeline = format!(
            "pt=96 config-interval=-1 ! udpsink host=127.0.0.1 port={} sync=false async=false",
            video.local_addr()?.port()
        );
        let system = std::env::var_os("SystemRoot")
            .ok_or_else(|| std::io::Error::other("Windows directory is unavailable"))?;
        let private_path =
            std::env::join_paths([runtime.to_path_buf(), Path::new(&system).join("System32")])
                .map_err(std::io::Error::other)?;
        let job = Job::new()?;
        let stop_event = StopEvent::new()?;
        let disconnect_event = StopEvent::new()?;
        let disconnect_handle = disconnect_event.handle.0 as usize;
        let mut child = Command::new(exe)
            .current_dir(runtime)
            .args([
                "-n",
                name,
                "-s",
                &format!("{width}x{height}"),
                "-fps",
                &fps.to_string(),
                "-vrtp",
                &video_pipeline,
                "-as",
                "0",
                "-vs",
                "fakesink",
                "-nofreeze",
            ])
            .env("PATH", private_path)
            .env("IMIRROR_STOP_EVENT", &stop_event.name)
            .env("IMIRROR_DISCONNECT_EVENT", &disconnect_event.name)
            .env("GST_PLUGIN_PATH", runtime.join("lib/gstreamer-1.0"))
            .env("GST_PLUGIN_PATH_1_0", runtime.join("lib/gstreamer-1.0"))
            .env("GST_PLUGIN_SYSTEM_PATH", "")
            .env("GST_PLUGIN_SYSTEM_PATH_1_0", "")
            .env(
                "GST_REGISTRY_1_0",
                std::env::temp_dir().join("imirror-gstreamer-registry.bin"),
            )
            .env("GST_REGISTRY_FORK", "no")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()?;
        if let Err(error) = job.assign(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let mut receiver = Self {
            child,
            stop,
            threads: Vec::new(),
            _job: Some(job),
            stop_event: Some(stop_event),
            disconnect_event: Some(disconnect_event),
        };
        if let Some(stdout) = receiver.child.stdout.take() {
            receiver.threads.push(
                thread::Builder::new()
                    .name("airplay-stdout".into())
                    .spawn(move || drain_log(stdout))?,
            );
        }
        if let Some(stderr) = receiver.child.stderr.take() {
            receiver.threads.push(
                thread::Builder::new()
                    .name("airplay-stderr".into())
                    .spawn(move || drain_log(stderr))?,
            );
        }
        let stopped = receiver.stop.clone();
        let video_session = session.clone();
        receiver
            .threads
            .push(
                thread::Builder::new()
                    .name("airplay-video".into())
                    .spawn(move || {
                        let mut parser = H264Assembler::default();
                        let mut sink = MediaFoundationSink::new(video_session.clone());
                        let mut bytes = vec![0u8; 65536];
                        while !stopped.load(Ordering::Acquire) {
                            // SAFETY: Receiver owns this event until all its workers join.
                            // The helper signals it only when the last client disconnects.
                            let disconnected = unsafe {
                                use windows::Win32::System::Threading::{
                                    ResetEvent, WaitForSingleObject,
                                };
                                let handle = HANDLE(disconnect_handle as *mut _);
                                if WaitForSingleObject(handle, 0)
                                    == windows::Win32::Foundation::WAIT_OBJECT_0
                                {
                                    let _ = ResetEvent(handle);
                                    true
                                } else {
                                    false
                                }
                            };
                            if disconnected {
                                let _ = video_session.reset_encoded();
                                parser = H264Assembler::default();
                                sink = MediaFoundationSink::new(video_session.clone());
                            }
                            match video.recv(&mut bytes) {
                                Ok(length) => {
                                    if let Ok(Some(frame)) = parser.push(&bytes[..length]) {
                                        match sink.submit(&frame) {
                                            Ok(()) => {}
                                            Err(DecodeError::MissingConfiguration) => {}
                                            Err(_) => {
                                                tracing::warn!("AirPlay encoded frame rejected")
                                            }
                                        }
                                    }
                                }
                                Err(e)
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::TimedOut
                                            | std::io::ErrorKind::WouldBlock
                                    ) => {}
                                Err(_) => break,
                            }
                        }
                    })?,
            );
        Ok(receiver)
    }

    pub fn stop(&mut self) -> bool {
        self.stop.store(true, Ordering::Release);
        if let Some(event) = &self.stop_event {
            event.signal();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let graceful = loop {
            match self.child.try_wait() {
                Ok(Some(status)) => break status.success(),
                Err(_) => break false,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                Ok(None) => break false,
            }
        };
        if !graceful {
            let _ = self.child.kill();
        }
        drop(self._job.take());
        let _ = self.child.wait();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        self.stop_event = None;
        self.disconnect_event = None;
        graceful
    }

    pub fn failure(&mut self) -> Option<String> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(format!("AirPlay receiver exited ({status}).")),
            Err(_) => Some("Could not query the AirPlay receiver.".into()),
            Ok(None) => None,
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}
/// Drain fixed buffers even if a helper writes a very long line. Only known
/// event categories enter logs; authentication values and arbitrary peer text
/// are never persisted.
fn drain_log(mut input: impl Read) {
    let mut bytes = [0u8; 2048];
    while let Ok(length) = input.read(&mut bytes) {
        if length == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&bytes[..length]).to_ascii_lowercase();
        if text.contains("error") || text.contains("failed") {
            tracing::warn!("AirPlay receiver reported an error");
        } else if text.contains("gstreamer") {
            tracing::debug!("AirPlay media pipeline diagnostic");
        }
    }
}
