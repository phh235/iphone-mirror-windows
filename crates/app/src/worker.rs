use crossbeam_channel::{Receiver, Sender, bounded};
use imirror_device::Reconnect;
use imirror_native_core::{CaptureOptions, Device, Engine, Status};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub enum Command {
    Connect(Device, usize),
    AirPlay(usize),
    Disconnect,
    Refresh,
    Rotate(usize, i32),
    Configure(imirror_device::Config, usize),
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub devices: Vec<Device>,
    pub status: Status,
    pub error: String,
    pub active: bool,
    pub decoder_mode: &'static str,
}
pub struct Worker {
    pub commands: Sender<Command>,
    pub snapshots: Receiver<Snapshot>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn start(mut config: imirror_device::Config) -> std::io::Result<Self> {
        let (commands, incoming) = bounded(16);
        let (outgoing, snapshots) = bounded(1);
        let replace = snapshots.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::Builder::new()
            .name("imirror-device".into())
            .spawn(move || {
                let mut snapshot = Snapshot::default();
                let engine = match Engine::new() {
                    Ok(e) => e,
                    Err(error) => {
                        snapshot.error = error.to_string();
                        let _ = outgoing.send(snapshot);
                        return;
                    }
                };
                let mut session: Option<Arc<imirror_native_core::Session>> = None;
                let mut receiver: Option<imirror_video_airplay::Receiver> = None;
                let mut airplay_wanted: Option<usize> = None;
                let mut desired: Option<(Device, usize)> = None;
                let mut retry = Reconnect::default();
                let mut scan_at = Instant::now();
                let mut metadata = true;
                let mut rotation = 0;
                while !stopping.load(Ordering::Acquire) {
                    if let Ok(command) = incoming.recv_timeout(Duration::from_millis(100)) {
                        match command {
                            Command::Configure(updated, hwnd) => {
                                let restart_airplay = airplay_wanted.is_some()
                                    && (config.receiver_name != updated.receiver_name
                                        || config.quality != updated.quality);
                                config = updated;
                                if restart_airplay {
                                    receiver = None;
                                    session = None;
                                    retry.request(Instant::now());
                                }
                                if let Some(s) = &session {
                                    let (w, h, fps) =
                                        crate::settings::render_request(config.quality);
                                    let result = s.video_preferences(w, h, fps).and_then(|_| {
                                        // SAFETY: UI retains the preview until worker shutdown.
                                        unsafe {
                                            s.view_preferences(
                                                hwnd as *mut _,
                                                config.one_to_one,
                                                config.vsync,
                                            )
                                        }
                                    });
                                    if let Err(error) = result {
                                        snapshot.error = error.to_string();
                                    }
                                }
                            }
                            Command::AirPlay(hwnd) => {
                                receiver = None;
                                session = None;
                                desired = None;
                                airplay_wanted = Some(hwnd);
                                snapshot.status = Status::default();
                                snapshot.error.clear();
                                retry.request(Instant::now());
                            }
                            Command::Connect(device, hwnd) => {
                                receiver = None;
                                airplay_wanted = None;
                                session = None;
                                snapshot.status = Status::default();
                                snapshot.error.clear();
                                desired = Some((device, hwnd));
                                retry.request(Instant::now());
                            }
                            Command::Disconnect => {
                                receiver = None;
                                airplay_wanted = None;
                                desired = None;
                                session = None;
                                retry.disconnect();
                                snapshot.status = Status::default();
                                snapshot.error.clear();
                                scan_at = Instant::now();
                            }
                            Command::Refresh => {
                                scan_at = Instant::now();
                                metadata = true;
                            }
                            Command::Rotate(hwnd, turns) => {
                                rotation = turns;
                                if let Some(s) = &session {
                                    // SAFETY: UI keeps its HWND alive until worker shutdown has completed.
                                    if let Err(error) =
                                        unsafe { s.rotate(hwnd as *mut _, rotation) }
                                    {
                                        snapshot.error = error.to_string();
                                    }
                                }
                            }
                        }
                    }
                    let mut session_lost = false;
                    if let Some(s) = &session {
                        snapshot.decoder_mode = s.decoder_mode();
                        match s.status() {
                            Ok(status) => {
                                if status.state == 4 && status.source_frames > 0 {
                                    retry.connected();
                                }
                                if matches!(status.state, 6 | 7) {
                                    session_lost = true;
                                }
                                snapshot.status = status;
                            }
                            Err(error) => {
                                snapshot.error = error.to_string();
                                session_lost = true;
                            }
                        }
                    }
                    if let Some(receiver) = &mut receiver
                        && let Some(error) = receiver.failure()
                    {
                        snapshot.error = error;
                        session_lost = true;
                    }
                    if session_lost {
                        receiver = None;
                        session = None;
                        retry.lost(Instant::now());
                        if !config.reconnect {
                            retry.disconnect();
                        }
                        scan_at = Instant::now();
                        metadata = true;
                        if snapshot.status.failure_kind == 3 {
                            retry.disconnect();
                        }
                    }
                    if session.is_none() {
                        if Instant::now() >= scan_at {
                            match engine.devices(metadata) {
                                Ok(devices) => snapshot.devices = devices,
                                Err(error) => snapshot.error = error.to_string(),
                            }
                            metadata = false;
                            scan_at = Instant::now() + Duration::from_secs(2);
                        }
                        if retry.ready(Instant::now())
                            && let Some((wanted, hwnd)) = &desired
                        {
                            if let Some(device) =
                                snapshot.devices.iter().find(|d| d.id == wanted.id)
                            {
                                let mut options = CaptureOptions::default();
                                (options.width, options.height, options.fps) =
                                    crate::settings::render_request(config.quality);
                                match engine.connect(device, options) {
                                    Ok(connected) => {
                                        // SAFETY: UI retains the preview until this worker joins.
                                        match unsafe { connected.attach(*hwnd as *mut _) } {
                                            Ok(()) => {
                                                // SAFETY: Same attached window and lifetime as above.
                                                let _ = unsafe {
                                                    let _ = connected.view_preferences(
                                                        *hwnd as *mut _,
                                                        config.one_to_one,
                                                        config.vsync,
                                                    );
                                                    connected.rotate(*hwnd as *mut _, rotation)
                                                };
                                                session = Some(Arc::new(connected));
                                                snapshot.error.clear();
                                            }
                                            Err(error) => {
                                                snapshot.error = error.to_string();
                                                retry.failed(Instant::now());
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        snapshot.error = error.to_string();
                                        // Driver changes need an explicit setup action, not repeated retries.
                                        if matches!(error.code, -9 | -1) {
                                            retry.disconnect();
                                        } else {
                                            retry.failed(Instant::now());
                                        }
                                    }
                                }
                            } else {
                                retry.failed(Instant::now());
                                snapshot.error =
                                    "Waiting for the selected iPhone. Reconnect USB and unlock it."
                                        .into();
                            }
                        }
                    }
                    if session.is_none()
                        && retry.ready(Instant::now())
                        && let Some(hwnd) = airplay_wanted
                    {
                        let start: Result<_, Box<dyn std::error::Error>> = (|| {
                            let connected = Arc::new(engine.encoded(CaptureOptions::default())?);
                            // SAFETY: UI retains the child window until this worker and receiver have stopped.
                            unsafe {
                                connected.attach(hwnd as *mut _)?;
                                connected.rotate(hwnd as *mut _, rotation)?;
                                connected.view_preferences(
                                    hwnd as *mut _,
                                    config.one_to_one,
                                    config.vsync,
                                )?;
                            }
                            let executable = std::env::current_exe()?;
                            let runtime = executable
                                .parent()
                                .ok_or("Executable directory is unavailable")?
                                .join("AirPlay");
                            let (width, height, fps) =
                                crate::settings::airplay_request(config.quality);
                            let receiver = imirror_video_airplay::Receiver::start(
                                connected.clone(),
                                &runtime,
                                &config.receiver_name,
                                width,
                                height,
                                fps,
                            )?;
                            Ok((connected, receiver))
                        })(
                        );
                        match start {
                            Ok((connected, active_receiver)) => {
                                session = Some(connected);
                                receiver = Some(active_receiver);
                                snapshot.error.clear();
                            }
                            Err(error) => {
                                snapshot.error = error.to_string();
                                retry.failed(Instant::now());
                            }
                        }
                    }
                    if !config.reconnect && session.is_none() && !snapshot.error.is_empty() {
                        retry.disconnect();
                    }
                    snapshot.active = session.is_some();
                    if outgoing.is_full() {
                        let _ = replace.try_recv();
                    }
                    let _ = outgoing.try_send(snapshot.clone());
                }
                drop(receiver);
                drop(session);
            })?;
        Ok(Self {
            commands,
            snapshots,
            stop,
            thread: Some(thread),
        })
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}
