//! Cached subscribed clients and connection timing. Built/refreshed only on the control path.
use super::{BluetoothError, Bounded};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use windows::{
    Devices::Bluetooth::{BluetoothLEDevice, GenericAttributeProfile::*},
    Foundation::TypedEventHandler,
    core::{HSTRING, IInspectable},
};
pub(crate) struct Peer {
    pub mouse: GattSubscribedClient,
    pub keyboard: Option<GattSubscribedClient>,
    pub active: Arc<AtomicBool>,
    interval_us: Arc<AtomicU64>,
    session: GattSession,
    session_token: i64,
    device: Option<BluetoothLEDevice>,
    interval_token: Option<i64>,
    epoch: Arc<super::subscription::Epoch>,
    generation: u64,
}
impl Peer {
    pub fn new(
        mouse: GattSubscribedClient,
        keyboard: Option<GattSubscribedClient>,
        id: &str,
        wake: Arc<dyn Fn() + Send + Sync>,
        epoch: Arc<super::subscription::Epoch>,
        generation: u64,
    ) -> Result<Self, BluetoothError> {
        let session = mouse.Session()?;
        let active = Arc::new(AtomicBool::new(
            session.SessionStatus()? == GattSessionStatus::Active,
        ));
        let observed = active.clone();
        let changed = wake.clone();
        let changed_epoch = epoch.clone();
        let token = session.SessionStatusChanged(&TypedEventHandler::<
            GattSession,
            GattSessionStatusChangedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let _ = args.Status()?;
                // A session becoming Active again must not resurrect an old CCCD/client.
                observed.store(false, Ordering::Release);
                changed_epoch.invalidate();
                changed();
            }
            Ok(())
        }))?;
        let interval_us = Arc::new(AtomicU64::new(0));
        let device = BluetoothLEDevice::FromIdAsync(&HSTRING::from(id))
            .and_then(|op| op.bounded())
            .ok();
        let mut interval_token = None;
        if let Some(device) = &device {
            if let Ok(interval) = device
                .GetConnectionParameters()
                .and_then(|p| p.ConnectionInterval())
            {
                interval_us.store(u64::from(interval) * 1250, Ordering::Relaxed);
            }
            let interval = interval_us.clone();
            interval_token = device
                .ConnectionParametersChanged(
                    &TypedEventHandler::<BluetoothLEDevice, IInspectable>::new(move |device, _| {
                        if let Some(device) = device.as_ref()
                            && let Ok(value) = device
                                .GetConnectionParameters()
                                .and_then(|p| p.ConnectionInterval())
                        {
                            interval.store(u64::from(value) * 1250, Ordering::Release);
                            wake();
                        }
                        Ok(())
                    }),
                )
                .ok();
        }
        Ok(Self {
            mouse,
            keyboard,
            active,
            interval_us,
            session,
            session_token: token,
            device,
            interval_token,
            epoch,
            generation,
        })
    }
    pub fn interval_us(&self) -> Option<u64> {
        let us = self.interval_us.load(Ordering::Acquire);
        (us > 0).then_some(us)
    }
    pub fn valid(&self) -> bool {
        self.active.load(Ordering::Acquire) && self.epoch.matches(self.generation)
    }
    pub fn invalidate(&self) {
        self.active.store(false, Ordering::Release);
        self.epoch.invalidate();
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.session.RemoveSessionStatusChanged(self.session_token);
        if let (Some(device), Some(token)) = (&self.device, self.interval_token) {
            let _ = device.RemoveConnectionParametersChanged(token);
        }
    }
}
