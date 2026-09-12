//! Signaled waits and a per-thread high-resolution timer; no global timer-resolution change.
use std::{sync::Arc, time::Duration};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Threading::*,
    },
    core::PCWSTR,
};
pub struct Signal(HANDLE);
// SAFETY: A kernel event is thread-safe. Arc retains the handle until all senders stop.
unsafe impl Send for Signal {}
// SAFETY: SetEvent/Wait operations are synchronized by the Windows kernel.
unsafe impl Sync for Signal {}
impl Signal {
    pub fn new() -> windows::core::Result<Arc<Self>> {
        // SAFETY: No borrowed security attributes/name; handle ownership is transferred to Self.
        unsafe {
            Ok(Arc::new(Self(CreateEventW(
                None,
                false,
                false,
                PCWSTR::null(),
            )?)))
        }
    }
    pub fn pulse(&self) {
        // SAFETY: Arc ownership keeps the event valid.
        unsafe {
            let _ = SetEvent(self.0);
        }
    }
}
impl Drop for Signal {
    fn drop(&mut self) {
        // SAFETY: Last Arc owner closes the owned handle once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
pub struct Waiter {
    signal: Arc<Signal>,
    timer: HANDLE,
    pub high_resolution: bool,
}
impl Waiter {
    pub fn new(signal: Arc<Signal>) -> windows::core::Result<Self> {
        // SAFETY: Timer is owned here, unnamed, and has no callback pointers.
        unsafe {
            let high = CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            );
            let (timer, high_resolution) = match high {
                Ok(timer) => (timer, true),
                Err(_) => (
                    CreateWaitableTimerExW(None, PCWSTR::null(), 0, TIMER_ALL_ACCESS.0)?,
                    false,
                ),
            };
            Ok(Self {
                signal,
                timer,
                high_resolution,
            })
        }
    }
    pub fn wait(&self, duration: Duration) -> windows::core::Result<()> {
        let due = -((duration.as_nanos() / 100).clamp(1, i64::MAX as u128) as i64);
        // SAFETY: Both owned handles stay live; due is a relative 100ns interval; no APC callback.
        unsafe {
            SetWaitableTimer(self.timer, &due, 0, None, None, false)?;
            let result = WaitForMultipleObjects(&[self.signal.0, self.timer], false, u32::MAX);
            if result == windows::Win32::Foundation::WAIT_FAILED {
                return Err(windows::core::Error::from_win32());
            }
        }
        Ok(())
    }
}
impl Drop for Waiter {
    fn drop(&mut self) {
        // SAFETY: Timer waits have ended before worker-local destruction.
        unsafe {
            let _ = CancelWaitableTimer(self.timer);
            let _ = CloseHandle(self.timer);
        }
    }
}
