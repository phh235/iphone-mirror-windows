//! Control orchestration; Raw Input publishes into a coalescing mailbox, never this command queue.
use crate::{
    low_latency::{Signal, Waiter},
    raw_input::RawInput,
};
use crossbeam_channel::{Receiver, Sender, bounded};
use imirror_coordinate_map::Size;
use imirror_input_ble::{DiagnosticAction, Diagnostics, HidPeripheral};
use imirror_input_core::{
    Controller, Input,
    metrics::{Metrics, now_ns},
    relative::{Mailbox, Packet},
};
use imirror_input_wda::Wda;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows::Win32::Foundation::{HWND, RECT};

pub enum Command {
    WdaConnect,
    RefreshGeometry,
    BleStart,
    BleSelect(String),
    BleDiagnostic(DiagnosticAction),
    PointerSpeed(u16),
    Disable,
    Action(Input),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Off,
    BluetoothMouse,
    Wda,
}
#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    pub home: bool,
    pub keyboard: bool,
    pub mouse: bool,
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
    pub pointer_speed_percent: u16,
    pub pointer_settings_notice: Option<String>,
    pub performance: imirror_input_core::metrics::Snapshot,
    pub connection_interval_us: Option<u64>,
    pub pacing_interval_us: u64,
    pub high_resolution_wait: bool,
    pub captured: bool,
    pub emergency_shortcut_available: bool,
    pub ready: bool,
    pub transition_queue_depth: usize,
    pub diagnostics_error: Option<String>,
}
pub struct ControlManager {
    commands: Sender<Command>,
    pub snapshots: Receiver<Snapshot>,
    pub mailbox: Arc<Mailbox>,
    raw: Option<RawInput>,
    raw_error: Option<String>,
    backend: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    log_thread: Option<JoinHandle<()>>,
    pub diagnostics: Arc<crate::diagnostics::Store>,
}
impl ControlManager {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let signal = Signal::new()?;
        let wake_signal = signal.clone();
        let metrics = Arc::new(Metrics::default());
        let mailbox = Arc::new(Mailbox::new(metrics, Arc::new(move || wake_signal.pulse())));
        let (raw, raw_error) = match RawInput::start(mailbox.clone()) {
            Ok(raw) => (Some(raw), None),
            Err(error) => (None, Some(error)),
        };
        let hotkey = raw.as_ref().is_some_and(RawInput::hotkey_ready);
        let invalidation = raw
            .as_ref()
            .map(RawInput::invalidation_handler)
            .unwrap_or_else(|| {
                let input = mailbox.clone();
                Arc::new(move || input.invalidate())
            });
        let (commands, incoming) = bounded::<Command>(16);
        let (outgoing, snapshots) = bounded(1);
        let replace = snapshots.clone();
        let (log_tx, log_rx) = bounded::<Snapshot>(1);
        let (log_thread, diagnostics) = crate::diagnostics::start_writer(log_rx)?;
        let diagnostic_state = diagnostics.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let backend = Arc::new(AtomicU8::new(0));
        let selected_backend = backend.clone();
        let input = mailbox.clone();
        let initial_error = raw_error.clone();
        let thread=thread::Builder::new().name("imirror-control".into()).spawn(move||{
            let mut snapshot=Snapshot{emergency_shortcut_available:hotkey,..Snapshot::default()};
            if let Some(error)=&initial_error{snapshot.ble_error=Some(error.clone());snapshot.message=error.clone();}
            let (speed,writable)=match crate::pointer_settings::load(){Ok(speed)=>(speed,true),Err(error)=>{snapshot.pointer_settings_notice=Some(error.to_string());(100,false)}};
            input.sensitivity.store(speed,Ordering::Relaxed);snapshot.pointer_speed_percent=speed;
            let _mta=match imirror_platform_windows::Mta::new(){Ok(mta)=>mta,Err(error)=>{snapshot.message=error.to_string();let _=outgoing.send(snapshot);return;}};
            let waiter=match Waiter::new(signal.clone()){Ok(waiter)=>waiter,Err(error)=>{snapshot.message=error.to_string();let _=outgoing.send(snapshot);return;}};
            snapshot.high_resolution_wait=waiter.high_resolution;
            let mut ble:Option<HidPeripheral>=None;let mut wda:Option<Wda>=None;
            let mut ble_wanted=false;let mut retry_at=Instant::now();let mut diagnostics_at=Instant::now();let mut publish_at=Instant::now();let mut save_at:Option<Instant>=None;
            let mut next_movement=0u64;let mut cadence_us=7_500u64;let mut completion_ema_us=0u64;
            while !stopping.load(Ordering::Acquire){
                while let Ok(command)=incoming.try_recv(){
                    let result=(||->Result<(),String>{match command{
                        Command::BleStart=>{
                            if let Some(error)=&initial_error{return Err(error.clone());}
                            input.release();wda=None;snapshot.geometry=None;ble_wanted=true;retry_at=Instant::now();snapshot.ble_error=None;snapshot.mode=1;selected_backend.store(1,Ordering::Release);
                        },
                        Command::WdaConnect=>{
                            input.release();input.ready.store(false,Ordering::Release);ble_wanted=false;ble=None;wda=None;snapshot.mode=0;snapshot.geometry=None;selected_backend.store(0,Ordering::Release);
                            let mut client=Wda::new("http://127.0.0.1:8100").map_err(|e|e.to_string())?;
                            snapshot.geometry=Some(client.geometry().map_err(|e|e.to_string())?);wda=Some(client);snapshot.mode=2;selected_backend.store(2,Ordering::Release);input.ready.store(true,Ordering::Release);snapshot.message="Advanced control connected".into();
                        },
                        Command::RefreshGeometry=>{if let Some(wda)=&mut wda{snapshot.geometry=Some(wda.geometry().map_err(|e|e.to_string())?);}},
                        Command::BleSelect(id)=>{input.release();ble.as_mut().ok_or("Control is off")?.select(&id).map_err(|e|e.to_string())?;},
                        Command::BleDiagnostic(action)=>{let result=ble.as_mut().ok_or("Control is off")?.diagnostic_action(action).map_err(|e|e.to_string());snapshot.last_diagnostic=Some(match &result{Ok(())=>format!("{}: notification accepted; physical result needs confirmation",action.name()),Err(e)=>e.clone()});result?;},
                        Command::PointerSpeed(speed)=>{input.sensitivity.store(speed.clamp(5,200),Ordering::Relaxed);save_at=Some(Instant::now()+Duration::from_millis(400));},
                        Command::Disable=>{
                            input.release();input.ready.store(false,Ordering::Release);input.enabled.store(false,Ordering::Release);
                            if let Some(ble)=&mut ble{ble.release();}
                            if let Some(wda)=&mut wda{let _=wda.dispatch(Input::Release);}
                            ble=None;wda=None;ble_wanted=false;snapshot.mode=0;snapshot.ble=Diagnostics::default();snapshot.ble_clients.clear();snapshot.message="Control is off".into();selected_backend.store(0,Ordering::Release);
                        },
                        Command::Action(action)=>{wda.as_mut().ok_or("Advanced control is not connected")?.dispatch(action).map_err(|e|e.to_string())?;}
                    }Ok(())})();
                    if let Err(error)=result{if selected_backend.load(Ordering::Acquire)==2{input.ready.store(false,Ordering::Release);snapshot.geometry=None;}snapshot.message=error.clone();snapshot.ble_error=Some(error);}
                }
                if ble_wanted && ble.is_none() && Instant::now()>=retry_at{
                    match HidPeripheral::start(&mut snapshot.ble){Ok(mut service)=>{let wake=signal.clone();service.set_waker(Arc::new(move||wake.pulse()));service.set_invalidation_handler(invalidation.clone());ble=Some(service);diagnostics_at=Instant::now();snapshot.ble_error=None;},Err(error)=>{snapshot.ble_error=Some(error.to_string());snapshot.message=error.to_string();retry_at=Instant::now()+Duration::from_secs(3);}}
                }
                if let Some(service)=&mut ble{
                    if Instant::now()>=diagnostics_at{
                        diagnostics_at=Instant::now()+Duration::from_millis(500);
                        if let Err(error)=service.validate_subscriptions(){service.invalidate();snapshot.ble_error=Some(error.to_string());}
                        match service.diagnostics(){Ok(mut d)=>{
                            if d.mouse_subscribers.len()==1 && !service.cached_ready() && d.advertising_status=="STARTED" && d.enumeration.protocol_mode==1 && !d.enumeration.suspended{
                                let id=d.mouse_subscribers[0].clone();match service.select(&id){Ok(())=>{d.selected_target=Some(id);snapshot.ble_error=None;},Err(error)=>snapshot.ble_error=Some(error.to_string())}
                            }
                            snapshot.ble_clients=d.mouse_subscribers.clone();
                            snapshot.message=if service.cached_ready(){"Control ready. Click the video to capture; Ctrl+Alt+Q releases.".into()}else if d.advertising_status=="STARTED"{"Pair this PC with your iPhone in Bluetooth settings, then enable AssistiveTouch.".into()}else{d.guidance().into()};snapshot.ble=d;
                        },Err(error)=>{snapshot.ble_error=Some(error.to_string());service.invalidate();}}
                    }
                    let ready=service.cached_ready() && hotkey;
                    if input.ready.swap(ready,Ordering::AcqRel)&&!ready{input.release();}
                    // An invalidation callback racing the publication must win.
                    if !service.cached_ready(){input.ready.store(false,Ordering::Release);}
                    snapshot.connection_interval_us=service.connection_interval_us();
                    // Negotiated interval is authoritative. Completion may mean Windows queueing.
                    // If the API is unavailable, use at least the BLE minimum plus observed completion,
                    // mark the fallback in diagnostics, and require a physical stability check.
                    cadence_us=snapshot.connection_interval_us.unwrap_or(completion_ema_us.saturating_mul(2).max(7_500));
                    if ready {
                        let now=now_ns();let stale_age=cadence_us.clamp(8_000,50_000)*1000;
                        if let Some(packet)=input.take(now,now>=next_movement,stale_age){
                            if !input.captured.load(Ordering::Acquire) && !packet.is_neutral(){continue;}
                            let timing=packet.timing();if timing.received!=0{input.metrics.queue.record(now.saturating_sub(timing.queued));}
                            let started=now_ns();
                            let result=match packet{
                                Packet::Button{previous,buttons,dx,dy,wheel,..}=>{
                                    let movement=if dx!=0||dy!=0 {service.mouse_cached(previous,dx,dy,0,||input.metrics.submitted(timing.received,now_ns(),false,false))}else{Ok(())};
                                    // Never suppress UP because a movement send failed ambiguously.
                                    let buttons=if input.captured.load(Ordering::Acquire){buttons}else{0};
                                    let transition=service.mouse_cached(buttons,0,0,wheel,||{let at=now_ns();input.metrics.submitted(timing.button,at,true,false);if timing.wheel!=0{input.metrics.wheel.record(at.saturating_sub(timing.wheel));}});
                                    movement.and(transition)
                                },
                                Packet::Mouse{buttons,dx,dy,wheel,..}=>service.mouse_cached(buttons,dx,dy,wheel,||{
                                    let at=now_ns();input.metrics.submitted(timing.received,at,false,false);
                                    if timing.button!=0{input.metrics.buttons.record(at.saturating_sub(timing.button));}
                                    if timing.wheel!=0{input.metrics.wheel.record(at.saturating_sub(timing.wheel));}
                                }),
                                Packet::Keyboard{modifiers,keys,..}=>service.keyboard_cached(modifiers,&keys,||{}),
                            };
                            let completed=now_ns();let elapsed=(completed-started)/1000;completion_ema_us=if completion_ema_us==0{elapsed}else{(completion_ema_us*7+elapsed)/8};
                            if matches!(packet,Packet::Mouse{..}|Packet::Button{..}){next_movement=completed+cadence_us*1000;}
                            match result{Ok(())=>{if timing.received!=0 && matches!(packet,Packet::Mouse{..}|Packet::Button{..}){input.metrics.complete.record(completed.saturating_sub(timing.received));}},Err(error)=>{input.metrics.failed.fetch_add(1,Ordering::Relaxed);snapshot.ble_error=Some(error.to_string());service.invalidate();diagnostics_at=Instant::now();}}
                            continue;
                        }
                    }
                }
                if save_at.is_some_and(|at|Instant::now()>=at){save_at=None;if writable{snapshot.pointer_settings_notice=crate::pointer_settings::save(input.sensitivity.load(Ordering::Relaxed)).err().map(|e|e.to_string());}}
                if Instant::now()>=publish_at{
                    publish_at=Instant::now()+Duration::from_secs(1);snapshot.performance=input.metrics.snapshot();snapshot.pointer_speed_percent=input.sensitivity.load(Ordering::Relaxed);snapshot.pacing_interval_us=cadence_us;snapshot.captured=input.captured.load(Ordering::Acquire);
                    snapshot.ready=input.ready.load(Ordering::Acquire);snapshot.transition_queue_depth=input.transition_depth();
                    snapshot.diagnostics_error=diagnostic_state.error.lock().unwrap_or_else(|e|e.into_inner()).clone();
                    if outgoing.is_full(){let _=replace.try_recv();}let _=outgoing.try_send(snapshot.clone());let _=log_tx.try_send(snapshot.clone());
                }
                let mut wait=publish_at.saturating_duration_since(Instant::now()).min(Duration::from_millis(100));
                if input.ready.load(Ordering::Acquire)&&input.has_pending(){wait=wait.min(Duration::from_nanos(next_movement.saturating_sub(now_ns()).max(1)));}
                if waiter.wait(wait).is_err(){input.release();break;}
            }
            input.release();input.ready.store(false,Ordering::Release);
            if let Some(ble)=&mut ble{ble.release();}
            if let Some(wda)=&mut wda{let _=wda.dispatch(Input::Release);}
            if writable{let _=crate::pointer_settings::save(input.sensitivity.load(Ordering::Relaxed));}
        })?;
        Ok(Self {
            commands,
            snapshots,
            mailbox,
            raw,
            raw_error,
            backend,
            stop,
            thread: Some(thread),
            log_thread: Some(log_thread),
            diagnostics,
        })
    }
    fn enqueue(&self, command: Command) -> bool {
        let sent = self.commands.try_send(command).is_ok();
        self.mailbox.wake();
        sent
    }
    pub fn enable(&self) -> bool {
        if self.raw_error.is_some() {
            return false;
        }
        self.mailbox.enabled.store(true, Ordering::Release);
        self.enqueue(Command::BleStart)
    }
    pub fn disable(&self) -> bool {
        self.release();
        self.mailbox.enabled.store(false, Ordering::Release);
        self.enqueue(Command::Disable)
    }
    pub fn is_ready(&self) -> bool {
        self.mailbox.ready.load(Ordering::Acquire)
    }
    pub fn backend(&self) -> Backend {
        match self.backend.load(Ordering::Acquire) {
            1 => Backend::BluetoothMouse,
            2 => Backend::Wda,
            _ => Backend::Off,
        }
    }
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            home: self.backend() == Backend::Wda && self.is_ready(),
            mouse: self.raw.is_some(),
            keyboard: self.raw.is_some() || self.backend() == Backend::Wda,
        }
    }
    pub fn capture(&self, owner: HWND, bounds: RECT) {
        if let Some(raw) = &self.raw {
            raw.capture(owner, bounds);
        }
    }
    pub fn release(&self) {
        if let Some(raw) = &self.raw {
            raw.release();
        } else {
            self.mailbox.release();
        }
    }
    pub fn send(&self, command: Command) -> bool {
        match command {
            Command::BleStart => self.enable(),
            Command::Disable => self.disable(),
            command => self.enqueue(command),
        }
    }
    pub fn stop(&mut self) {
        self.release();
        if let Some(raw) = &mut self.raw {
            raw.stop();
        }
        self.stop.store(true, Ordering::Release);
        self.mailbox.wake();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.log_thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for ControlManager {
    fn drop(&mut self) {
        self.stop();
    }
}
pub fn diagnostic_path() -> Option<std::path::PathBuf> {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(i) = args.iter().position(|a| a == "--ble-status-file") {
        return args.get(i + 1).map(std::path::PathBuf::from);
    }
    std::env::var_os("LOCALAPPDATA")
        .map(|root| std::path::PathBuf::from(root).join("iMirror/control-status.json"))
}
