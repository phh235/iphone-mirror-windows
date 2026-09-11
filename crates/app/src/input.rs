use crossbeam_channel::{Receiver, Sender, bounded};
use imirror_coordinate_map::Size;
use imirror_input_ble::{DiagnosticAction, Diagnostics, HidPeripheral};
use imirror_input_core::{Controller, Input, PointerScale};
use imirror_input_wda::Wda;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
pub enum Command {
    WdaConnect,
    RefreshGeometry,
    BleStart,
    BleSelect(String),
    BleDiagnostic(DiagnosticAction),
    PointerSpeed(u16),
    Disable,
    Action(Input),
    Mouse(u8, i32, i32, i32),
    Key(u8, Vec<u8>),
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub geometry: Option<Size>,
    pub message: String,
    pub ble_clients: Vec<String>,
    pub ble: Diagnostics,
    pub ble_error: Option<String>,
    pub last_diagnostic: Option<String>,
    pub mode: u8,
    pub latency_ms: Option<f64>,
    pub pointer_speed_percent: u16,
    pub pointer_settings_notice: Option<String>,
}
pub struct InputWorker {
    sender: Sender<(u64, Command)>,
    pub snapshots: Receiver<Snapshot>,
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl InputWorker {
    pub fn start() -> std::io::Result<Self> {
        let (sender, incoming) = bounded::<(u64, Command)>(32);
        let (outgoing, snapshots) = bounded(1);
        let replace = snapshots.clone();
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread=thread::Builder::new().name("imirror-input".into()).spawn(move|| {
            let mut motion=PointerScale::default();
            let (settings_writable,notice)=match crate::pointer_settings::load() {
                Ok(percent)=>{motion.set_percent(percent);(true,None)},
                Err(error)=>(false,Some(format!("Using 25% speed; preferences preserved: {error}"))),
            };
            let mut snapshot=Snapshot{pointer_speed_percent:motion.percent(),pointer_settings_notice:notice,..Snapshot::default()};
            let mut save_speed_at:Option<std::time::Instant>=None;
            let mut last_mouse_buttons:Option<u8>=None;
            let _mta=match imirror_platform_windows::Mta::new() {
                Ok(mta)=>mta,Err(error)=>{snapshot.message=error.to_string();let _=outgoing.send(snapshot);return;}
            };
            let mut wda:Option<Wda>=None; let mut ble:Option<HidPeripheral>=None;
            let mut last_generation=0;
            let mut last_diagnostics_json=String::new();
            // Explicit diagnostic invocation only; never armed by normal launch.
            let move_right_armed=std::env::args().any(|arg|arg=="--ble-move-right-once");
            let mut move_right_due:Option<std::time::Instant>=None;
            let mut move_right_attempted=false;
            while !stopping.load(Ordering::Acquire) {
                let gen_now=current.load(Ordering::Acquire);
                if gen_now!=last_generation {
                    motion.reset();last_mouse_buttons=None;
                    if let Some(ble)=&mut ble { ble.release(); }
                    if let Some(wda)=&mut wda { let _=wda.dispatch(Input::Release); }
                    last_generation=gen_now;
                }
                if let Ok((generation,command))=incoming.recv_timeout(Duration::from_millis(100)) {
                    if generation!=current.load(Ordering::Acquire) { continue; }
                    let start=std::time::Instant::now();
                    let result:Result<(),String>=(|| {
                        match command {
                            Command::WdaConnect=>{
                                motion.reset();last_mouse_buttons=None;
                                ble=None; wda=None; snapshot.mode=0; snapshot.geometry=None;
                                let mut client=Wda::new("http://127.0.0.1:8100").map_err(|e|e.to_string())?;
                                snapshot.geometry=Some(client.geometry().map_err(|e|e.to_string())?);
                                snapshot.mode=2; snapshot.message="WDA control ready".into(); wda=Some(client);
                            }
                            Command::RefreshGeometry=>{
                                snapshot.geometry=None;
                                if let Some(wda)=&mut wda {snapshot.geometry=Some(wda.geometry().map_err(|e|e.to_string())?);}
                            }
                            Command::BleStart=>{
                                motion.reset();last_mouse_buttons=None;
                                wda=None; ble=None; snapshot.geometry=None; snapshot.mode=0;
                                snapshot.ble_error=None; snapshot.last_diagnostic=None; snapshot.ble_clients.clear();
                                snapshot.message="Creating BLE HID service; waiting for advertising STARTED.".into();
                                if outgoing.is_full() { let _=replace.try_recv(); }
                                let _=outgoing.try_send(snapshot.clone());
                                ble=Some(HidPeripheral::start(&mut snapshot.ble).map_err(|e|e.to_string())?);
                                snapshot.mode=1;
                            }
                            Command::BleSelect(id)=>{
                                motion.reset();last_mouse_buttons=None;
                                ble.as_mut().ok_or("BLE is not enabled")?.select(&id).map_err(|e|e.to_string())?;
                                snapshot.ble_error=None;
                            }
                            Command::BleDiagnostic(action)=>{
                                let result=ble.as_mut().ok_or("BLE is not enabled")?
                                    .diagnostic_action(action).map_err(|e|e.to_string());
                                snapshot.last_diagnostic=Some(match &result {
                                    Ok(())=>format!("{}: Windows accepted the HID notification. Phone result requires your visual confirmation.",action.name()),
                                    Err(error)=>format!("{}: NOT CONFIRMED: {error}",action.name()),
                                });
                                snapshot.latency_ms=Some(start.elapsed().as_secs_f64()*1000.0);
                                result?;
                            }
                            Command::PointerSpeed(percent)=>{
                                motion.set_percent(percent);
                                snapshot.pointer_speed_percent=motion.percent();
                                if settings_writable {save_speed_at=Some(std::time::Instant::now()+Duration::from_millis(400));}
                            }
                            Command::Disable=>{
                                motion.reset();last_mouse_buttons=None;ble=None;wda=None;
                                snapshot=Snapshot{pointer_speed_percent:motion.percent(),pointer_settings_notice:snapshot.pointer_settings_notice.clone(),..Snapshot::default()};
                            }
                            Command::Action(action)=>{
                                let wda=wda.as_mut().ok_or("Enable WDA control first")?;
                                wda.dispatch(action).map_err(|e|e.to_string())?;
                                snapshot.latency_ms=wda.last_request.map(|d|d.as_secs_f64()*1000.0);
                            }
                            Command::Mouse(buttons,dx,dy,wheel)=>{
                                let (dx,dy)=motion.scale(dx,dy);
                                // Preserve button transitions/wheel, but do not enqueue zero-motion duplicates.
                                if dx!=0 || dy!=0 || wheel!=0 || last_mouse_buttons!=Some(buttons) {
                                    ble.as_mut().ok_or("Enable BLE control first")?.mouse(buttons,dx,dy,wheel).map_err(|e|e.to_string())?;
                                    last_mouse_buttons=Some(buttons);
                                    snapshot.latency_ms=Some(start.elapsed().as_secs_f64()*1000.0);
                                }
                            }
                            Command::Key(modifiers,keys)=>{
                                ble.as_mut().ok_or("Enable BLE control first")?.keyboard(modifiers,&keys).map_err(|e|e.to_string())?;
                                snapshot.latency_ms=Some(start.elapsed().as_secs_f64()*1000.0);
                            }
                        } Ok(())
                    })();
                    if let Err(error)=result {
                        motion.reset();last_mouse_buttons=None;
                        if wda.is_none() { snapshot.ble_error=Some(error.clone()); }
                        snapshot.message=error;
                    }
                }
                if let Some(ble)=&mut ble {
                    match ble.diagnostics() {
                        Ok(mut diagnostics)=>{
                            let clients=diagnostics.mouse_subscribers.clone();
                            // Selection alone never sends an input report.
                            if diagnostics.started_observed && diagnostics.selected_target.is_none()
                                && clients.len()==1 {
                                let id=clients[0].clone();
                                match ble.select(&id) {
                                    Ok(())=>{diagnostics.selected_target=Some(id);snapshot.ble_error=None;}
                                    Err(error)=>snapshot.ble_error=Some(error.to_string()),
                                }
                            }
                            if move_right_armed && !move_right_attempted {
                                if diagnostics.mouse_ready() {
                                    let due=move_right_due.get_or_insert_with(||std::time::Instant::now()+Duration::from_secs(1));
                                    if std::time::Instant::now()>=*due {
                                        // Mark attempted BEFORE dispatch: ambiguous failures are never replayed.
                                        move_right_attempted=true;
                                        let result=ble.diagnostic_action(DiagnosticAction::MoveRight);
                                        snapshot.last_diagnostic=Some(match result {
                                            Ok(())=>"Move Right: one +40 X report accepted by Windows. Physical movement awaits user confirmation.".into(),
                                            Err(error)=>format!("Move Right: single attempt failed ({error}); not retried."),
                                        });
                                    }
                                } else {move_right_due=None;}
                            }
                            snapshot.ble_clients=clients;
                            snapshot.message=diagnostics.guidance().into();
                            snapshot.ble=diagnostics;
                        }
                        Err(error)=>{
                            snapshot.ble.mouse_subscribers.clear();
                            snapshot.ble.keyboard_subscribers.clear();
                            snapshot.ble_clients.clear();
                            snapshot.ble_error=Some(error.to_string());
                        }
                    }
                    if let Some(error)=&snapshot.ble_error { snapshot.message=error.clone(); }
                }
                if save_speed_at.is_some_and(|due|std::time::Instant::now()>=due) {
                    save_speed_at=None;
                    snapshot.pointer_settings_notice=crate::pointer_settings::save(motion.percent()).err().map(|error|format!("Speed is applied but could not be saved: {error}"));
                }
                // BLE state changes only; no video/audio data or per-frame I/O.
                if let Ok(json)=serde_json::to_string_pretty(&serde_json::json!({
                    "process_id":std::process::id(), "ble":snapshot.ble,
                    "executable":std::env::current_exe().ok(),
                    "pointer_speed_percent":snapshot.pointer_speed_percent,"pointer_settings_notice":snapshot.pointer_settings_notice,
                    "error":snapshot.ble_error, "last_diagnostic":snapshot.last_diagnostic,
                })) && json!=last_diagnostics_json {
                    if let Some(path)=diagnostic_path() {
                        let written=(|| -> std::io::Result<()> {
                            if let Some(parent)=path.parent() { std::fs::create_dir_all(parent)?; }
                            std::fs::write(&path,&json)
                        })();
                        if let Err(error)=written { snapshot.message=format!("BLE diagnostics file could not be written: {error}"); }
                    }
                    last_diagnostics_json=json;
                }
                if outgoing.is_full() { let _=replace.try_recv(); }
                let _=outgoing.try_send(snapshot.clone());
            }
            if save_speed_at.is_some() && settings_writable {let _=crate::pointer_settings::save(motion.percent());}
            if let Some(ble)=&mut ble { ble.release(); }
            if let Some(wda)=&mut wda { let _=wda.dispatch(Input::Release); }
        })?;
        Ok(Self {
            sender,
            snapshots,
            generation,
            stop,
            thread: Some(thread),
        })
    }
    pub fn send(&self, command: Command) -> bool {
        if self
            .sender
            .try_send((self.generation.load(Ordering::Acquire), command))
            .is_err()
        {
            self.cancel();
            false
        } else {
            true
        }
    }
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
    pub fn stop(&mut self) {
        self.cancel();
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for InputWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Local inspection only; no network service and no pairing keys are persisted.
pub fn diagnostic_path() -> Option<std::path::PathBuf> {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--ble-status-file") {
        return args.get(index + 1).map(std::path::PathBuf::from);
    }
    std::env::var_os("LOCALAPPDATA")
        .map(|base| std::path::PathBuf::from(base).join("iMirror/ble-status.json"))
}
