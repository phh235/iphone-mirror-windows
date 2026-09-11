use serde::Serialize;
use windows::{
    Win32::{
        Foundation::ERROR_SUCCESS,
        Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1},
        System::{
            Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW},
            WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        },
    },
    core::{PCWSTR, w},
};
/// COM apartments must be initialized and uninitialized on the same thread.
pub struct Mta(std::marker::PhantomData<std::rc::Rc<()>>);
impl Mta {
    pub fn new() -> windows::core::Result<Self> {
        // SAFETY: This thread balances each successful RoInitialize with one RoUninitialize.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self(std::marker::PhantomData))
    }
}
impl Drop for Mta {
    fn drop(&mut self) {
        // SAFETY: Mta is !Send and represents a successful initialization on this thread.
        unsafe {
            RoUninitialize();
        }
    }
}
fn registry_string(key: PCWSTR, name: PCWSTR) -> Option<String> {
    let mut data = [0u16; 512];
    let mut size = std::mem::size_of_val(&data) as u32;
    // SAFETY: Output has size writable bytes; only REG_SZ is accepted.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key,
            name,
            RRF_RT_REG_SZ,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    Some(String::from_utf16_lossy(
        &data[..data.iter().position(|c| *c == 0).unwrap_or(data.len())],
    ))
}
#[derive(Debug, Serialize)]
pub struct Host {
    pub windows_build: Option<String>,
    pub cpu: Option<String>,
    pub gpus: Vec<String>,
    pub bluetooth: Option<imirror_input_ble::Capability>,
    pub bluetooth_error: Option<String>,
}
pub fn detect() -> windows::core::Result<Host> {
    let _mta = Mta::new()?;
    let mut gpus = Vec::new();
    // SAFETY: DXGI returns owned COM interfaces. Each descriptor is copied before release.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        for index in 0..32 {
            let Ok(adapter) = factory.EnumAdapters1(index) else {
                break;
            };
            let description = adapter.GetDesc1()?;
            let chars = &description.Description;
            gpus.push(String::from_utf16_lossy(
                &chars[..chars.iter().position(|c| *c == 0).unwrap_or(chars.len())],
            ));
        }
    }
    let (bluetooth, bluetooth_error) = match imirror_input_ble::capability() {
        Ok(caps) => (Some(caps), None),
        Err(error) => (None, Some(error.to_string())),
    };
    Ok(Host {
        windows_build: registry_string(
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
        ),
        cpu: registry_string(
            w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
            w!("ProcessorNameString"),
        ),
        gpus,
        bluetooth,
        bluetooth_error,
    })
}
