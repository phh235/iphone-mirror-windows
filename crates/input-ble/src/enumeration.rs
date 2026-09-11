//! Observable HID enumeration and connection events. No input capture or media code.
use super::{BluetoothError, Bounded, HidProfile, buffer, characteristic};
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    Devices::{
        Bluetooth::{
            BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice,
            GenericAttributeProfile::*,
        },
        Enumeration::{DeviceInformation, DeviceInformationUpdate, DeviceWatcher},
    },
    Foundation::TypedEventHandler,
    Security::Cryptography::CryptographicBuffer,
    core::IInspectable,
};
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub unix_ms: u128,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub paired: Option<bool>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Observation {
    pub protocol_mode: u8,
    pub suspended: bool,
    pub reads: BTreeMap<String, u64>,
    pub report_map_readers: Vec<String>,
    pub mouse_subscription_events: u64,
    pub keyboard_subscription_events: u64,
    pub touch_subscription_events: u64,
    pub gatt_sessions: BTreeMap<String, String>,
    pub connected_le: BTreeMap<String, Peer>,
    pub connected_classic: BTreeMap<String, Peer>,
    pub le_scan_complete: bool,
    pub classic_scan_complete: bool,
    pub events: VecDeque<Event>,
}
impl Default for Observation {
    fn default() -> Self {
        Self {
            protocol_mode: 1,
            suspended: false,
            reads: BTreeMap::new(),
            report_map_readers: Vec::new(),
            mouse_subscription_events: 0,
            keyboard_subscription_events: 0,
            touch_subscription_events: 0,
            gatt_sessions: BTreeMap::new(),
            connected_le: BTreeMap::new(),
            connected_classic: BTreeMap::new(),
            le_scan_complete: false,
            classic_scan_complete: false,
            events: VecDeque::new(),
        }
    }
}
impl Observation {
    fn event(&mut self, detail: String) {
        if self.events.len() == 64 {
            self.events.pop_front();
        }
        self.events.push_back(Event {
            unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            detail,
        });
    }
}
struct SessionWatch {
    id: String,
    session: GattSession,
    token: i64,
}
impl Drop for SessionWatch {
    fn drop(&mut self) {
        let _ = self.session.RemoveSessionStatusChanged(self.token);
    }
}
#[derive(Default)]
struct Inner {
    observation: Mutex<Observation>,
    sessions: Mutex<Vec<SessionWatch>>,
}
#[derive(Clone, Default)]
pub(crate) struct Trace(Arc<Inner>);
impl Trace {
    pub fn snapshot(&self) -> Observation {
        self.0
            .observation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn note(&self, detail: impl Into<String>) {
        self.0
            .observation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .event(detail.into());
    }
    pub fn observe_session(&self, session: GattSession) -> windows::core::Result<()> {
        let id = session.DeviceId()?.Id()?.to_string();
        let status = session.SessionStatus()?.0;
        {
            let mut state = self.0.observation.lock().unwrap_or_else(|e| e.into_inner());
            if state.gatt_sessions.len() < 32 || state.gatt_sessions.contains_key(&id) {
                state
                    .gatt_sessions
                    .insert(id.clone(), format!("GattSessionStatus({status})"));
            }
        }
        // Register before retaining it. Callback only uses a Weak reference, so
        // COM event ownership cannot keep the service alive after shutdown.
        let sessions = self.0.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if sessions.len() >= 32 || sessions.iter().any(|s| s.id == id) {
            return Ok(());
        }
        drop(sessions);
        let weak = Arc::downgrade(&self.0);
        let event_id = id.clone();
        let token = session.SessionStatusChanged(&TypedEventHandler::<
            GattSession,
            GattSessionStatusChangedEventArgs,
        >::new(move |_, args| {
            if let (Some(inner), Some(args)) = (weak.upgrade(), args.as_ref()) {
                let status = args.Status()?.0;
                let error = args.Error()?.0;
                let mut state = inner.observation.lock().unwrap_or_else(|e| e.into_inner());
                state
                    .gatt_sessions
                    .insert(event_id.clone(), format!("GattSessionStatus({status})"));
                state.event(format!(
                    "GattSessionStatusChanged: {event_id}: status={status}, error={error}"
                ));
            }
            Ok(())
        }))?;
        let watch = SessionWatch { id, session, token };
        let mut sessions = self.0.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if sessions.len() < 32 && !sessions.iter().any(|s| s.id == watch.id) {
            sessions.push(watch);
        }
        Ok(())
    }
    pub fn touch_discovery_confirmed(&self, target: &str) -> bool {
        let state = self.0.observation.lock().unwrap_or_else(|e| e.into_inner());
        state.protocol_mode == 1
            && !state.suspended
            && state.report_map_readers.iter().any(|id| id == target)
    }
    pub fn read(&self, name: &str, offset: u32, reader: Option<String>) {
        let mut state = self.0.observation.lock().unwrap_or_else(|e| e.into_inner());
        if name == "Report Map 0x2A4B"
            && let Some(id) = reader
            && state.report_map_readers.len() < 32
            && !state.report_map_readers.contains(&id)
        {
            state.report_map_readers.push(id);
        }
        let count = state.reads.entry(name.into()).or_default();
        *count = count.saturating_add(1);
        state.event(format!("ReadRequested: {name}; offset={offset}"));
    }
    pub fn subscribers(
        &self,
        report_id: u8,
        characteristic: &GattLocalCharacteristic,
        profile: HidProfile,
    ) -> windows::core::Result<()> {
        let mut ids = Vec::new();
        for client in characteristic.SubscribedClients()? {
            let session = client.Session()?;
            ids.push(session.DeviceId()?.Id()?.to_string());
            if let Err(error) = self.observe_session(session) {
                self.note(format!("Subscription session observation failed: {error}"));
            }
        }
        let mut state = self.0.observation.lock().unwrap_or_else(|e| e.into_inner());
        let count = if report_id == 2 && profile == HidProfile::DirectTouch {
            &mut state.touch_subscription_events
        } else if report_id == 2 {
            &mut state.mouse_subscription_events
        } else {
            &mut state.keyboard_subscription_events
        };
        *count = count.saturating_add(1);
        state.event(format!(
            "SubscribedClientsChanged: report ID {report_id}, count={}, clients={ids:?}",
            ids.len()
        ));
        Ok(())
    }
}
pub(crate) fn install_read(
    characteristic: &GattLocalCharacteristic,
    value: Arc<Mutex<Vec<u8>>>,
    name: &'static str,
    trace: Trace,
) -> windows::core::Result<i64> {
    characteristic.ReadRequested(&TypedEventHandler::<
        GattLocalCharacteristic,
        GattReadRequestedEventArgs,
    >::new(move |_, args| {
        let Some(args) = args.as_ref() else {
            return Ok(());
        };
        let deferral = args.GetDeferral()?;
        let result = (|| {
            let request = args.GetRequestAsync()?.bounded()?;
            let reader = args
                .Session()
                .and_then(|session| session.DeviceId())
                .and_then(|id| id.Id())
                .ok()
                .map(|id| id.to_string());
            trace.read(name, request.Offset()?, reader);
            if let Err(error) = args
                .Session()
                .and_then(|session| trace.observe_session(session))
            {
                trace.note(format!("Read session observation failed: {error}"));
            }
            let bytes = value.lock().unwrap_or_else(|e| e.into_inner()).clone();
            // Match upstream: Windows receives the full attribute value and handles ATT framing.
            request.RespondWithValue(&buffer(&bytes)?)
        })();
        if let Err(error) = &result {
            trace.note(format!("ReadRequested failed for {name}: {error}"));
        }
        deferral.Complete()?;
        result
    }))
}
#[derive(Clone, Copy)]
pub(crate) enum WriteKind {
    None,
    ControlPoint,
    ProtocolMode,
}
pub(crate) struct Metadata {
    characteristic: GattLocalCharacteristic,
    read_token: Option<i64>,
    write_token: Option<i64>,
}
pub(crate) fn hid_byte(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [value @ 0..=1] => Some(*value),
        _ => None,
    }
}
impl Metadata {
    pub fn new(
        service: &GattLocalService,
        id: u16,
        name: &'static str,
        initial: &[u8],
        kind: WriteKind,
        trace: Trace,
        encrypted: bool,
    ) -> Result<Self, BluetoothError> {
        let properties = match kind {
            WriteKind::None => {
                if id == 0x2a19 {
                    GattCharacteristicProperties::Read | GattCharacteristicProperties::Notify
                } else {
                    GattCharacteristicProperties::Read
                }
            }
            WriteKind::ControlPoint => GattCharacteristicProperties::WriteWithoutResponse,
            WriteKind::ProtocolMode => {
                GattCharacteristicProperties::Read
                    | GattCharacteristicProperties::WriteWithoutResponse
            }
        };
        let encrypted = encrypted && id != 0x2a19;
        let characteristic = characteristic(service, id, properties, None, encrypted)?;
        let mut metadata = Self {
            characteristic,
            read_token: None,
            write_token: None,
        };
        let value = Arc::new(Mutex::new(initial.to_vec()));
        if !matches!(kind, WriteKind::ControlPoint) {
            metadata.read_token = Some(install_read(
                &metadata.characteristic,
                value.clone(),
                name,
                trace.clone(),
            )?);
        }
        if !matches!(kind, WriteKind::None) {
            let event_trace = trace.clone();
            metadata.write_token = Some(metadata.characteristic.WriteRequested(
                &TypedEventHandler::<GattLocalCharacteristic, GattWriteRequestedEventArgs>::new(
                    move |_, args| {
                        let Some(args) = args.as_ref() else {
                            return Ok(());
                        };
                        let deferral = args.GetDeferral()?;
                        let result = (|| {
                            let request = args.GetRequestAsync()?.bounded()?;
                            if let Err(error) = args
                                .Session()
                                .and_then(|session| event_trace.observe_session(session))
                            {
                                event_trace
                                    .note(format!("Write session observation failed: {error}"));
                            }
                            let payload = request.Value()?;
                            let mut bytes = windows::core::Array::<u8>::new();
                            if payload.Length()? == 1 {
                                CryptographicBuffer::CopyToByteArray(&payload, &mut bytes)?;
                            }
                            let command = if request.Offset()? == 0 {
                                hid_byte(&bytes)
                            } else {
                                None
                            };
                            if let Some(command) = command {
                                *value.lock().unwrap_or_else(|e| e.into_inner()) = vec![command];
                                let mut state = event_trace
                                    .0
                                    .observation
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner());
                                match kind {
                                    WriteKind::ProtocolMode => state.protocol_mode = command,
                                    WriteKind::ControlPoint => state.suspended = command == 0,
                                    WriteKind::None => {}
                                }
                                state.event(format!("WriteRequested: {name} = {command}"));
                            } else {
                                event_trace.note(format!(
                                    "Rejected malformed {name} write (length/offset/value)"
                                ));
                            }
                            if request.Option()? == GattWriteOption::WriteWithResponse {
                                if command.is_some() {
                                    request.Respond()?;
                                } else {
                                    request.RespondWithProtocolError(0x13)?;
                                }
                            }
                            Ok(())
                        })();
                        if let Err(error) = &result {
                            event_trace.note(format!("WriteRequested failed for {name}: {error}"));
                        }
                        deferral.Complete()?;
                        result
                    },
                ),
            )?);
        }
        trace.note(format!(
            "Created {name} (0x{id:04X}); properties=0x{:X}; {}",
            properties.0,
            if encrypted {
                "encrypted access"
            } else {
                "plain metadata"
            }
        ));
        Ok(metadata)
    }
}
impl Drop for Metadata {
    fn drop(&mut self) {
        if let Some(token) = self.read_token {
            let _ = self.characteristic.RemoveReadRequested(token);
        }
        if let Some(token) = self.write_token {
            let _ = self.characteristic.RemoveWriteRequested(token);
        }
    }
}
struct PeerWatcher {
    watcher: DeviceWatcher,
    added: i64,
    removed: i64,
    completed: i64,
}
impl PeerWatcher {
    fn start(le: bool, trace: Trace) -> windows::core::Result<Self> {
        let selector = if le {
            BluetoothLEDevice::GetDeviceSelectorFromConnectionStatus(
                BluetoothConnectionStatus::Connected,
            )?
        } else {
            BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
                BluetoothConnectionStatus::Connected,
            )?
        };
        let watcher = DeviceInformation::CreateWatcherAqsFilter(&selector)?;
        let added_trace = trace.clone();
        let added = watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
            move |_, info| {
                if let Some(info) = info.as_ref() {
                    let peer = Peer {
                        id: info.Id()?.to_string(),
                        name: info.Name()?.to_string(),
                        paired: info.Pairing().and_then(|p| p.IsPaired()).ok(),
                    };
                    let mut state = added_trace
                        .0
                        .observation
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    state.event(format!(
                        "Bluetooth connected ({}): {}",
                        if le { "LE" } else { "Classic" },
                        peer.name
                    ));
                    let peers = if le {
                        &mut state.connected_le
                    } else {
                        &mut state.connected_classic
                    };
                    if peers.len() < 32 {
                        peers.insert(peer.id.clone(), peer);
                    }
                }
                Ok(())
            },
        ))?;
        let removed_trace = trace.clone();
        let removed = match watcher.Removed(&TypedEventHandler::<
            DeviceWatcher,
            DeviceInformationUpdate,
        >::new(move |_, info| {
            if let Some(info) = info.as_ref() {
                let id = info.Id()?.to_string();
                let mut state = removed_trace
                    .0
                    .observation
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let peers = if le {
                    &mut state.connected_le
                } else {
                    &mut state.connected_classic
                };
                peers.remove(&id);
                state.event(format!(
                    "Bluetooth disconnected ({}): {id}",
                    if le { "LE" } else { "Classic" }
                ));
            }
            Ok(())
        })) {
            Ok(token) => token,
            Err(error) => {
                let _ = watcher.RemoveAdded(added);
                return Err(error);
            }
        };
        let completed = match watcher.EnumerationCompleted(&TypedEventHandler::<
            DeviceWatcher,
            IInspectable,
        >::new(move |_, _| {
            let mut state = trace
                .0
                .observation
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if le {
                state.le_scan_complete = true;
            } else {
                state.classic_scan_complete = true;
            }
            Ok(())
        })) {
            Ok(token) => token,
            Err(error) => {
                let _ = watcher.RemoveAdded(added);
                let _ = watcher.RemoveRemoved(removed);
                return Err(error);
            }
        };
        let registered = Self {
            watcher,
            added,
            removed,
            completed,
        };
        registered.watcher.Start()?;
        Ok(registered)
    }
}
impl Drop for PeerWatcher {
    fn drop(&mut self) {
        let _ = self.watcher.Stop();
        let _ = self.watcher.RemoveAdded(self.added);
        let _ = self.watcher.RemoveRemoved(self.removed);
        let _ = self.watcher.RemoveEnumerationCompleted(self.completed);
    }
}
pub(crate) struct Connections {
    _le: Option<PeerWatcher>,
    _classic: Option<PeerWatcher>,
}
impl Connections {
    pub fn start(trace: Trace) -> Self {
        let start = |le| match PeerWatcher::start(le, trace.clone()) {
            Ok(w) => Some(w),
            Err(e) => {
                trace.note(format!("Connection watcher unavailable (LE={le}): {e}"));
                None
            }
        };
        Self {
            _le: start(true),
            _classic: start(false),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_values_are_bounded() {
        assert_eq!(hid_byte(&[0]), Some(0));
        assert_eq!(hid_byte(&[1]), Some(1));
        for bytes in [&[][..], &[2][..], &[1, 0][..]] {
            assert_eq!(hid_byte(bytes), None);
        }
    }
    #[test]
    fn event_history_is_bounded_without_inventing_subscriptions() {
        let trace = Trace::default();
        for i in 0..500 {
            trace.note(format!("event {i}"));
        }
        let s = trace.snapshot();
        assert_eq!(s.events.len(), 64);
        assert_eq!(s.protocol_mode, 1);
        assert_eq!(s.mouse_subscription_events, 0);
        assert_eq!(s.keyboard_subscription_events, 0);
        assert!(s.gatt_sessions.is_empty());
    }
}
