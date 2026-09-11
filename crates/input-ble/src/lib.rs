//! WinRT HID-over-GATT. Report design adapted from MIT windows-ble-hid.
//! Copyright (c) 2026 Abhishek Raj; upstream notice is in vendor/licenses.
mod enumeration;
use enumeration::{Connections, Metadata, Trace, WriteKind, install_read};
pub use enumeration::{Event, Observation, Peer};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use windows::{
    Devices::Bluetooth::{
        BluetoothAdapter, BluetoothError as WinBluetoothError, GenericAttributeProfile::*,
    },
    Foundation::TypedEventHandler,
    Security::Cryptography::CryptographicBuffer,
    Storage::Streams::IBuffer,
    core::GUID,
};

const REPORT_MAP: &[u8] = &[
    0x05, 0x01, 0x09, 0x06, 0xa1, 0x01, 0x85, 0x01, 0x05, 0x07, 0x19, 0xe0, 0x29, 0xe7, 0x15, 0x00,
    0x25, 0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x06,
    0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xc0, 0x05,
    0x01, 0x09, 0x02, 0xa1, 0x01, 0x85, 0x02, 0x09, 0x01, 0xa1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29,
    0x03, 0x15, 0x00, 0x25, 0x01, 0x95, 0x03, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x05, 0x81,
    0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x16, 0x01, 0x80, 0x26, 0xff, 0x7f, 0x75, 0x10, 0x95,
    0x02, 0x81, 0x06, 0x09, 0x38, 0x15, 0x81, 0x25, 0x7f, 0x75, 0x08, 0x95, 0x01, 0x81, 0x06, 0xc0,
    0xc0,
];
fn uuid(id: u16) -> GUID {
    GUID::from_u128(((id as u128) << 96) | 0x00001000800000805f9b34fb)
}
fn buffer(bytes: &[u8]) -> windows::core::Result<IBuffer> {
    CryptographicBuffer::CreateFromByteArray(bytes)
}
#[derive(Debug, thiserror::Error)]
pub enum BluetoothError {
    #[error("Bluetooth adapter does not support BLE HID peripheral mode.")]
    Unsupported,
    #[error("Bluetooth API failed: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("Bluetooth service registration failed: {name} ({0})", name = bluetooth_error_name(*.0))]
    Registration(i32),
    #[error("Select a paired iPhone subscribed to BLE HID before sending input.")]
    NoTarget,
    #[error("The selected Bluetooth report subscriber is unavailable.")]
    Disconnected,
    #[error("The selected target has not subscribed to the keyboard report.")]
    NoKeyboardSubscriber,
}
fn bluetooth_error_name(code: i32) -> &'static str {
    match code {
        0 => "Success",
        1 => "RadioNotAvailable - Windows cannot access the Bluetooth radio",
        2 => "ResourceInUse - another service owns the resource",
        3 => "DeviceNotConnected",
        4 => "OtherError",
        5 => "DisabledByPolicy",
        6 => "NotSupported",
        7 => "DisabledByUser",
        8 => "ConsentRequired",
        9 => "TransportNotSupported",
        _ => "Unknown Bluetooth error",
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Capability {
    pub adapter_name: String,
    pub adapter_id: String,
    pub computer_name: String,
    pub radio_state: String,
    pub low_energy: bool,
    pub peripheral: bool,
}
/// Call on the input worker's initialized MTA, never the UI thread.
pub fn capability() -> Result<Capability, BluetoothError> {
    let adapter = BluetoothAdapter::GetDefaultAsync()?.bounded()?;
    let id = adapter.DeviceId()?;
    let name = windows::Devices::Enumeration::DeviceInformation::CreateFromIdAsync(&id)
        .and_then(|op| op.bounded())
        .and_then(|info| info.Name())
        .map(|name| name.to_string())
        .unwrap_or_else(|_| "Name unavailable".into());
    let radio_state = adapter
        .GetRadioAsync()
        .and_then(|op| op.bounded())
        .and_then(|radio| radio.State())
        .map(|state| {
            match state.0 {
                1 => "ON",
                2 => "OFF",
                3 => "DISABLED",
                _ => "UNKNOWN",
            }
            .to_string()
        })
        .unwrap_or_else(|error| format!("UNAVAILABLE: {error}"));
    Ok(Capability {
        radio_state,
        adapter_name: name,
        adapter_id: id.to_string(),
        computer_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Name unavailable".into()),
        low_energy: adapter.IsLowEnergySupported()?,
        peripheral: adapter.IsPeripheralRoleSupported()?,
    })
}
/// Observed OS state, never inferred from a successful start request.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Diagnostics {
    pub adapter: Option<Capability>,
    pub hid_service_created: bool,
    pub advertising_status: String,
    pub advertising_error: Option<i32>,
    pub started_observed: bool,
    pub startup_timed_out: bool,
    /// Only subscribers whose GATT session is currently Active.
    pub mouse_subscribers: Vec<String>,
    pub keyboard_subscribers: Vec<String>,
    pub stored_mouse_subscriptions: u32,
    pub stored_keyboard_subscriptions: u32,
    pub selected_target: Option<String>,
    pub enumeration: Observation,
    pub gap_appearance: String,
    pub cccd_support: String,
}
impl Diagnostics {
    pub fn mouse_ready(&self) -> bool {
        self.started_observed
            && self.enumeration.protocol_mode == 1
            && !self.enumeration.suspended
            && self
                .selected_target
                .as_ref()
                .is_some_and(|target| self.mouse_subscribers.contains(target))
    }
    pub fn keyboard_ready(&self) -> bool {
        self.mouse_ready()
            && self
                .selected_target
                .as_ref()
                .is_some_and(|target| self.keyboard_subscribers.contains(target))
    }
    pub fn guidance(&self) -> &'static str {
        if self.mouse_ready() {
            "Mouse report subscriber selected. Use Move Right; phone movement is not yet confirmed."
        } else if self.advertising_status == "STARTED" {
            if self.mouse_subscribers.is_empty() {
                "Advertising STARTED. Bluetooth Connected is not HID ready. Waiting for iOS to read HID metadata and subscribe to the mouse report."
            } else {
                "Mouse report subscriber detected. Select the iPhone connection before sending input."
            }
        } else if self.startup_timed_out {
            "Advertising did not reach STARTED within 10 seconds. Check the status/error; control is not ready."
        } else if self.started_observed {
            "HID advertising is no longer STARTED. Check the current status; no automatic restart is performed."
        } else {
            "Starting BLE HID advertising; waiting for Windows to report STARTED."
        }
    }
}
#[derive(Default)]
struct Advertisement {
    status: i32,
    error: Option<i32>,
    started: bool,
}
fn advertisement_name(status: i32) -> String {
    match status {
        0 => "CREATED".into(),
        1 => "STOPPED".into(),
        2 => "STARTED".into(),
        3 => "ABORTED".into(),
        4 => "STARTED_WITHOUT_ALL_ADVERTISEMENT_DATA".into(),
        other => format!("UNKNOWN({other})"),
    }
}
#[derive(Clone, Copy, Debug)]
pub enum DiagnosticAction {
    MoveRight,
    MoveLeft,
    LeftClick,
    TypeA,
}
impl DiagnosticAction {
    pub fn name(self) -> &'static str {
        match self {
            Self::MoveRight => "Move Right (+40 relative X)",
            Self::MoveLeft => "Move Left (-40 relative X)",
            Self::LeftClick => "Left Click (down/up)",
            Self::TypeA => "Type A (Shift+A down/up)",
        }
    }
}
pub fn keyboard_report(modifiers: u8, keys: &[u8]) -> [u8; 8] {
    let mut report = [0; 8];
    report[0] = modifiers;
    if keys.len() > 6 {
        report[2..].fill(1);
    } else {
        report[2..2 + keys.len()].copy_from_slice(keys);
    }
    report
}
pub fn mouse_report(buttons: u8, dx: i32, dy: i32, wheel: i32) -> [u8; 6] {
    let x = (dx.clamp(-32767, 32767) as i16).to_le_bytes();
    let y = (dy.clamp(-32767, 32767) as i16).to_le_bytes();
    [
        buttons & 7,
        x[0],
        x[1],
        y[0],
        y[1],
        wheel.clamp(-127, 127) as i8 as u8,
    ]
}
fn characteristic(
    service: &GattLocalService,
    id: u16,
    properties: GattCharacteristicProperties,
    static_value: Option<&[u8]>,
    encrypted: bool,
) -> Result<GattLocalCharacteristic, BluetoothError> {
    let params = GattLocalCharacteristicParameters::new()?;
    params.SetCharacteristicProperties(properties)?;
    let protection = if encrypted {
        GattProtectionLevel::EncryptionRequired
    } else {
        GattProtectionLevel::Plain
    };
    if properties.contains(GattCharacteristicProperties::Read) {
        params.SetReadProtectionLevel(protection)?;
    }
    if properties.contains(GattCharacteristicProperties::Write)
        || properties.contains(GattCharacteristicProperties::WriteWithoutResponse)
    {
        params.SetWriteProtectionLevel(protection)?;
    }
    if let Some(value) = static_value {
        params.SetStaticValue(&buffer(value)?)?;
    }
    let result = service
        .CreateCharacteristicAsync(uuid(id), &params)?
        .bounded()?;
    let error = result.Error()?;
    if error != WinBluetoothError::Success {
        return Err(BluetoothError::Registration(error.0));
    }
    Ok(result.Characteristic()?)
}
struct Report {
    characteristic: GattLocalCharacteristic,
    value: Arc<Mutex<Vec<u8>>>,
    read_token: i64,
    subscription_token: i64,
}
impl Report {
    fn new(
        service: &GattLocalService,
        id: u8,
        size: usize,
        trace: Trace,
    ) -> Result<Self, BluetoothError> {
        let characteristic = characteristic(
            service,
            0x2a4d,
            GattCharacteristicProperties::Read | GattCharacteristicProperties::Notify,
            None,
            true,
        )?;
        let value = Arc::new(Mutex::new(vec![0; size]));
        let name = if id == 2 {
            "Mouse Input Report 0x2A4D"
        } else {
            "Keyboard Input Report 0x2A4D"
        };
        let read_token = install_read(&characteristic, value.clone(), name, trace.clone())?;
        let event_trace = trace.clone();
        let subscription_token =
            match characteristic.SubscribedClientsChanged(&TypedEventHandler::<
                GattLocalCharacteristic,
                windows::core::IInspectable,
            >::new(move |sender, _| {
                if let Some(sender) = sender.as_ref()
                    && let Err(error) = event_trace.subscribers(id, sender)
                {
                    event_trace.note(format!("Subscription observation failed: {error}"));
                }
                Ok(())
            })) {
                Ok(token) => token,
                Err(error) => {
                    let _ = characteristic.RemoveReadRequested(read_token);
                    return Err(error.into());
                }
            };
        let report = Self {
            characteristic,
            value,
            read_token,
            subscription_token,
        };
        let descriptor = GattLocalDescriptorParameters::new()?;
        descriptor.SetReadProtectionLevel(GattProtectionLevel::EncryptionRequired)?;
        descriptor.SetStaticValue(&buffer(&[id, 1])?)?;
        let result = report
            .characteristic
            .CreateDescriptorAsync(uuid(0x2908), &descriptor)?
            .bounded()?;
        if result.Error()? != WinBluetoothError::Success {
            return Err(BluetoothError::Registration(result.Error()?.0));
        }
        trace.note(format!("Created {name}: Read|Notify, encrypted read; Report Reference 0x2908=[{id},1]; CCCD supplied by Windows"));
        Ok(report)
    }
    fn send(&self, target: &str, bytes: &[u8]) -> Result<(), BluetoothError> {
        self.send_observed(target, bytes, || {})
    }
    fn send_observed(
        &self,
        target: &str,
        bytes: &[u8],
        mut before_notify: impl FnMut(),
    ) -> Result<(), BluetoothError> {
        *self.value.lock().unwrap_or_else(|e| e.into_inner()) = bytes.to_vec();
        for client in self.characteristic.SubscribedClients()? {
            let session = client.Session()?;
            if session.DeviceId()?.Id()? == target {
                if session.SessionStatus()? != GattSessionStatus::Active {
                    return Err(BluetoothError::Disconnected);
                }
                before_notify();
                let result = self
                    .characteristic
                    .NotifyValueForSubscribedClientAsync(&buffer(bytes)?, &client)?
                    .bounded()?;
                if result.Status()? == GattCommunicationStatus::Success {
                    return Ok(());
                }
                return Err(BluetoothError::Disconnected);
            }
        }
        Err(BluetoothError::Disconnected)
    }
}
impl Drop for Report {
    fn drop(&mut self) {
        let _ = self.characteristic.RemoveReadRequested(self.read_token);
        let _ = self
            .characteristic
            .RemoveSubscribedClientsChanged(self.subscription_token);
    }
}
/// Registration is explicit; constructing diagnostics never starts advertising.
/// Input is sent only to the explicitly selected subscriber, never broadcast.
pub struct HidPeripheral {
    provider: GattServiceProvider,
    battery: GattServiceProvider,
    keyboard: Report,
    mouse: Report,
    target: Option<String>,
    _metadata: Vec<Metadata>,
    _connections: Connections,
    trace: Trace,
    advertising_token: i64,
    advertisement: Arc<Mutex<Advertisement>>,
    requested_at: Instant,
    adapter: Capability,
}
impl HidPeripheral {
    pub fn start(diagnostics: &mut Diagnostics) -> Result<Self, BluetoothError> {
        *diagnostics = Diagnostics::default();
        diagnostics.advertising_status = "NOT_REQUESTED".into();
        let caps = capability()?;
        diagnostics.adapter = Some(caps.clone());
        if !caps.low_energy || !caps.peripheral {
            return Err(BluetoothError::Unsupported);
        }
        let result = GattServiceProvider::CreateAsync(uuid(0x1812))?.bounded()?;
        if result.Error()? != WinBluetoothError::Success {
            return Err(BluetoothError::Registration(result.Error()?.0));
        }
        let provider = result.ServiceProvider()?;
        diagnostics.hid_service_created = true;
        diagnostics.advertising_status = "CREATED".into();
        let service = provider.Service()?;
        let trace = Trace::default();
        trace.note(
            "Created HID service 0x1812; matching windows-ble-hid 9a4f451 characteristic order",
        );
        // Match upstream order and retain handlers for every readable/writable characteristic.
        let mut metadata = vec![
            Metadata::new(
                &service,
                0x2a4a,
                "HID Information 0x2A4A",
                &[0x11, 0x01, 0, 3],
                WriteKind::None,
                trace.clone(),
            )?,
            Metadata::new(
                &service,
                0x2a4b,
                "Report Map 0x2A4B",
                REPORT_MAP,
                WriteKind::None,
                trace.clone(),
            )?,
            Metadata::new(
                &service,
                0x2a4c,
                "HID Control Point 0x2A4C",
                &[],
                WriteKind::ControlPoint,
                trace.clone(),
            )?,
            Metadata::new(
                &service,
                0x2a4e,
                "Protocol Mode 0x2A4E",
                &[1],
                WriteKind::ProtocolMode,
                trace.clone(),
            )?,
        ];
        let keyboard = Report::new(&service, 1, 8, trace.clone())?;
        let mouse = Report::new(&service, 2, 6, trace.clone())?;
        let battery_result = GattServiceProvider::CreateAsync(uuid(0x180f))?.bounded()?;
        if battery_result.Error()? != WinBluetoothError::Success {
            return Err(BluetoothError::Registration(battery_result.Error()?.0));
        }
        let battery = battery_result.ServiceProvider()?;
        metadata.push(Metadata::new(
            &battery.Service()?,
            0x2a19,
            "Battery Level 0x2A19",
            &[100],
            WriteKind::None,
            trace.clone(),
        )?);
        let connections = Connections::start(trace.clone());
        let advertisement = Arc::new(Mutex::new(Advertisement::default()));
        let observed = advertisement.clone();
        let advertising_trace = trace.clone();
        let advertising_token =
            provider.AdvertisementStatusChanged(&TypedEventHandler::<
                GattServiceProvider,
                GattServiceProviderAdvertisementStatusChangedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let status = args.Status()?.0;
                    let error = args.Error()?.0;
                    let mut state = observed.lock().unwrap_or_else(|e| e.into_inner());
                    state.status = status;
                    state.error = (error != 0).then_some(error);
                    state.started |= status == GattServiceProviderAdvertisementStatus::Started.0;
                    advertising_trace.note(format!(
                        "AdvertisementStatusChanged: {} error={error}",
                        advertisement_name(status)
                    ));
                }
                Ok(())
            }))?;
        // Own the provider and event registration before requesting advertising:
        // a start error must still revoke callbacks and stop the service.
        let peripheral = Self {
            provider,
            battery,
            keyboard,
            mouse,
            target: None,
            _metadata: metadata,
            _connections: connections,
            trace,
            advertising_token,
            advertisement,
            requested_at: Instant::now(),
            adapter: caps,
        };
        let parameters = GattServiceProviderAdvertisingParameters::new()?;
        parameters.SetIsConnectable(true)?;
        parameters.SetIsDiscoverable(true)?;
        peripheral
            .provider
            .StartAdvertisingWithParameters(&parameters)?;
        diagnostics.advertising_status = "START_REQUESTED".into();
        Ok(peripheral)
    }
    pub fn diagnostics(&self) -> Result<Diagnostics, BluetoothError> {
        // The property confirms present OS state; the callback retains errors
        // and whether STARTED was ever observed between worker polls.
        let status = self.provider.AdvertisementStatus()?.0;
        let mut observed = self.advertisement.lock().unwrap_or_else(|e| e.into_inner());
        observed.status = status;
        observed.started |= status == GattServiceProviderAdvertisementStatus::Started.0;
        if status == GattServiceProviderAdvertisementStatus::Started.0 {
            observed.error = None;
        }
        let mut result = Diagnostics {
            adapter: Some(self.adapter.clone()),
            hid_service_created: true,
            advertising_status: advertisement_name(status),
            advertising_error: observed.error,
            started_observed: observed.started,
            startup_timed_out: !observed.started
                && self.requested_at.elapsed() >= Duration::from_secs(10),
            selected_target: self.target.clone(),
            enumeration: self.trace.snapshot(),
            gap_appearance: "Windows-managed GAP; public GATT APIs cannot override Appearance (same limitation as reference)".into(),
            cccd_support: "0x2902 auto-generated by Windows for both Read|Notify input reports".into(),
            ..Diagnostics::default()
        };
        drop(observed);
        result.mouse_subscribers = self.clients()?;
        result.stored_mouse_subscriptions =
            self.mouse.characteristic.SubscribedClients()?.Size()?;
        result.stored_keyboard_subscriptions =
            self.keyboard.characteristic.SubscribedClients()?.Size()?;
        for client in self.keyboard.characteristic.SubscribedClients()? {
            let session = client.Session()?;
            if session.SessionStatus()? != GattSessionStatus::Active {
                continue;
            }
            let id = session.DeviceId()?.Id()?.to_string();
            if !result.keyboard_subscribers.contains(&id) {
                result.keyboard_subscribers.push(id);
            }
        }
        result.mouse_subscribers.sort();
        result.keyboard_subscribers.sort();
        Ok(result)
    }
    pub fn diagnostic_action(&mut self, action: DiagnosticAction) -> Result<(), BluetoothError> {
        let state = self.diagnostics()?;
        if !state.mouse_ready() {
            return Err(BluetoothError::NoTarget);
        }
        match action {
            DiagnosticAction::MoveRight => self.mouse(0, 40, 0, 0),
            DiagnosticAction::MoveLeft => self.mouse(0, -40, 0, 0),
            DiagnosticAction::LeftClick => {
                let down = self.mouse(1, 0, 0, 0);
                std::thread::sleep(Duration::from_millis(40));
                let up = self.mouse(0, 0, 0, 0);
                down.and(up)
            }
            DiagnosticAction::TypeA => {
                if !state.keyboard_ready() {
                    return Err(BluetoothError::NoKeyboardSubscriber);
                }
                let down = self.keyboard(2, &[4]);
                std::thread::sleep(Duration::from_millis(40));
                let up = self.keyboard(0, &[]);
                down.and(up)
            }
        }
    }
    pub fn clients(&self) -> Result<Vec<String>, BluetoothError> {
        let mut result = Vec::new();
        for client in self.mouse.characteristic.SubscribedClients()? {
            let session = client.Session()?;
            if session.SessionStatus()? != GattSessionStatus::Active {
                continue;
            }
            let id = session.DeviceId()?.Id()?.to_string();
            if !result.contains(&id) {
                result.push(id);
            }
        }
        Ok(result)
    }
    pub fn select(&mut self, id: &str) -> Result<(), BluetoothError> {
        if !self.clients()?.iter().any(|s| s == id) {
            return Err(BluetoothError::NoTarget);
        }
        self.release();
        if !self.diagnostics()?.started_observed {
            return Err(BluetoothError::NoTarget);
        }
        self.target = Some(id.to_owned());
        self.trace
            .note(format!("Selected actual mouse report subscriber: {id}"));
        Ok(())
    }
    pub fn mouse(
        &mut self,
        buttons: u8,
        dx: i32,
        dy: i32,
        wheel: i32,
    ) -> Result<(), BluetoothError> {
        self.mouse_observed(buttons, dx, dy, wheel, || {})
    }
    pub fn mouse_observed(
        &mut self,
        buttons: u8,
        dx: i32,
        dy: i32,
        wheel: i32,
        before_notify: impl FnMut(),
    ) -> Result<(), BluetoothError> {
        let target = self.target.as_deref().ok_or(BluetoothError::NoTarget)?;
        self.mouse
            .send_observed(target, &mouse_report(buttons, dx, dy, wheel), before_notify)
    }
    pub fn keyboard(&mut self, modifiers: u8, keys: &[u8]) -> Result<(), BluetoothError> {
        let target = self.target.as_deref().ok_or(BluetoothError::NoTarget)?;
        self.keyboard
            .send(target, &keyboard_report(modifiers, keys))
    }
    pub fn release(&mut self) {
        if let Some(target) = &self.target {
            let _ = self.mouse.send(target, &[0; 6]);
            let _ = self.keyboard.send(target, &[0; 8]);
        }
    }
}
impl Drop for HidPeripheral {
    fn drop(&mut self) {
        self.release();
        let _ = self.provider.StopAdvertising();
        let _ = self
            .provider
            .RemoveAdvertisementStatusChanged(self.advertising_token);
        let _ = self.battery.StopAdvertising();
    }
}

trait Bounded<T> {
    fn bounded(&self) -> windows::core::Result<T>;
}
impl<T: windows::core::RuntimeType> Bounded<T> for windows_future::IAsyncOperation<T> {
    fn bounded(&self) -> windows::core::Result<T> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while self.Status()? == windows_future::AsyncStatus::Started {
            if std::time::Instant::now() >= deadline {
                let _ = self.Cancel();
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x800705b4u32 as i32,
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        self.GetResults()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_requires_observed_start_and_selected_real_subscriber() {
        let mut d = Diagnostics {
            selected_target: Some("phone".into()),
            ..Diagnostics::default()
        };
        d.mouse_subscribers.push("phone".into());
        assert!(!d.mouse_ready());
        assert!(!d.guidance().contains("Pair your iPhone"));
        d.advertising_status = "STARTED_WITHOUT_ALL_ADVERTISEMENT_DATA".into();
        assert!(!d.guidance().contains("Pair your iPhone"));
        d.started_observed = true;
        assert!(d.mouse_ready());
        d.enumeration.protocol_mode = 0;
        assert!(!d.mouse_ready());
        d.enumeration.protocol_mode = 1;
        d.enumeration.suspended = true;
        assert!(!d.mouse_ready());
        d.enumeration.suspended = false;
        assert!(d.mouse_ready());
        assert!(!d.keyboard_ready());
        d.keyboard_subscribers.push("different-host".into());
        assert!(!d.keyboard_ready());
        d.keyboard_subscribers.push("phone".into());
        assert!(d.keyboard_ready());
        d.mouse_subscribers.clear();
        d.stored_mouse_subscriptions = 1;
        d.stored_keyboard_subscriptions = 1;
        assert!(
            !d.mouse_ready(),
            "cached CCCD subscriptions do not imply an active link"
        );
        d.advertising_status = "ABORTED".into();
        assert!(!d.guidance().contains("Pair your iPhone"));
        d.advertising_status = "STARTED".into();
        assert!(d.guidance().contains("Waiting for iOS"));
    }
    #[test]
    fn reports_match_hogp_descriptor() {
        assert_eq!(mouse_report(1, -1, 256, -1), [1, 255, 255, 0, 1, 255]);
        assert_eq!(
            mouse_report(255, i32::MIN, i32::MAX, 999),
            [7, 1, 128, 255, 127, 127]
        );
        assert_eq!(keyboard_report(2, &[4]), [2, 0, 4, 0, 0, 0, 0, 0]);
        assert_eq!(keyboard_report(0, &[4; 7]), [0, 0, 1, 1, 1, 1, 1, 1]);
        assert_eq!(
            uuid(0x1812),
            GUID::from_u128(0x0000181200001000800000805f9b34fb)
        );
    }
}
