//! Explicit local benchmark bridge. Disabled unless IMIRROR_WDA_VISUAL_LAB is set.
//! Requests only observe our video; this bridge never sends input to a phone.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Component, PathBuf, Prefix},
    time::{Duration, Instant},
};
use windows::Win32::Foundation::HWND;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Observation {
    pts: i64,
    present_start: i64,
    present_end: i64,
    swap_chain: u64,
    changed_samples: u32,
    reserved: u32,
}
unsafe extern "C" {
    fn im_visual_probe_configure(window: u64, left: f64, top: f64, width: f64, height: f64) -> i32;
    fn im_visual_probe_arm() -> u64;
    fn im_visual_probe_read(generation: u64, output: *mut Observation, capacity: u32) -> i32;
    fn im_visual_probe_disable();
    fn im_frame_pacing_clock() -> i64;
    fn im_frame_pacing_frequency() -> i64;
}
pub struct Lab {
    directory: PathBuf,
    last_id: u64,
    next_poll: Instant,
}
impl Lab {
    pub fn from_env() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Some(value) = std::env::var_os("IMIRROR_WDA_VISUAL_LAB") else {
            return Ok(None);
        };
        let directory = PathBuf::from(value);
        if !directory.is_absolute()
            || !matches!(directory.components().next(),
            Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
        {
            return Err("Visual lab requires an absolute local-drive directory".into());
        }
        fs::create_dir_all(&directory)?;
        Ok(Some(Self {
            directory,
            last_id: 0,
            next_poll: Instant::now(),
        }))
    }
    pub fn poll(
        &mut self,
        window: HWND,
        video_live: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if Instant::now() < self.next_poll {
            return Ok(());
        }
        self.next_poll = Instant::now() + Duration::from_millis(100);
        let bytes = match fs::read(self.directory.join("request.json")) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        if bytes.len() > 4096 {
            return Err("Visual lab request exceeds 4 KiB".into());
        }
        let request: Value = serde_json::from_slice(&bytes)?;
        let id = request["id"]
            .as_u64()
            .filter(|id| *id != 0)
            .ok_or("Invalid visual lab id")?;
        if id == self.last_id {
            return Ok(());
        }
        self.last_id = id;
        let result = if request["action"] == "arm" && !video_live {
            Err("Video is not live; no visual measurement armed".into())
        } else {
            self.request(window, &request)
        };
        let reply = match result {
            Ok(value) => json!({"id":id,"ok":true,"value":value}),
            Err(error) => json!({"id":id,"ok":false,"error":error.to_string()}),
        };
        let temporary = self.directory.join("response.tmp");
        fs::write(&temporary, serde_json::to_vec(&reply)?)?;
        fs::rename(temporary, self.directory.join("response.json"))?;
        Ok(())
    }
    fn request(&self, window: HWND, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        match request["action"]
            .as_str()
            .ok_or("Missing visual lab action")?
        {
            "configure" => {
                let number = |key: &str| request[key].as_f64().ok_or("Missing ROI coordinate");
                let (left, top, width, height) = (
                    number("left")?,
                    number("top")?,
                    number("width")?,
                    number("height")?,
                );
                // SAFETY: Owned video HWND is only used as an identity; finite
                // normalized bounds are validated by the native probe.
                if unsafe {
                    im_visual_probe_configure(window.0 as usize as u64, left, top, width, height)
                } != 0
                {
                    return Err("Invalid video ROI".into());
                }
                Ok(json!({"configured":true,"pid":std::process::id()}))
            }
            "arm" => {
                let viewport = visible_viewport(window)?;
                // SAFETY: Native observer serializes state; no borrowed pointers.
                let generation = unsafe { im_visual_probe_arm() };
                if generation == u64::MAX {
                    return Err("ROI baseline does not match its initial reference image".into());
                }
                if generation == 0 {
                    return Err("ROI has no CPU-backed presented baseline yet".into());
                }
                // SAFETY: Read-only scalar QPC queries.
                Ok(unsafe {
                    json!({"generation":generation,"qpc":im_frame_pacing_clock(),
                    "frequency":im_frame_pacing_frequency(),"pid":std::process::id(),
                    "viewport":viewport})
                })
            }
            "read" => {
                let generation = request["generation"]
                    .as_u64()
                    .ok_or("Missing ROI generation")?;
                let mut observations = [Observation::default(); 256];
                // SAFETY: repr(C) matches native Observation; writable capacity
                // is exactly 256 records. Native read checks epoch and bounds.
                let count =
                    unsafe { im_visual_probe_read(generation, observations.as_mut_ptr(), 256) };
                if !(0..=256).contains(&count) {
                    return Err("ROI epoch unavailable, changed format, or overflow".into());
                }
                let events: Vec<_> = observations[..count as usize]
                    .iter()
                    .map(|o| {
                        json!({
                    "pts":o.pts,"present_start_qpc":o.present_start,"present_end_qpc":o.present_end,
                    "swap_chain":o.swap_chain,"changed_samples":o.changed_samples,
                    "render_width":o.reserved >> 16,"render_height":o.reserved & 0xffff})
                    })
                    .collect();
                Ok(json!({"generation":generation,"events":events,"limit":256,
                    "note":"ROI change on accepted Present; ETW correlation still required for display time."}))
            }
            "disable" => {
                // SAFETY: Synchronized observer disable; does not affect video.
                unsafe { im_visual_probe_disable() };
                Ok(json!({"disabled":true}))
            }
            _ => Err("Unknown visual lab action".into()),
        }
    }
}
fn visible_viewport(window: HWND) -> Result<[i32; 2], Box<dyn std::error::Error>> {
    use windows::Win32::{Foundation::RECT, UI::WindowsAndMessaging::*};
    let mut rect = RECT::default();
    // SAFETY: Caller owns this video child HWND on the UI thread. These are
    // read-only observations; no focus, sizing, or capture state is changed.
    unsafe {
        let root = GetAncestor(window, GA_ROOT);
        if root.0.is_null() || IsIconic(root).as_bool() || !IsWindowVisible(window).as_bool() {
            return Err("Benchmark window is hidden or minimized".into());
        }
        GetClientRect(window, &mut rect)?;
    }
    let size = [rect.right - rect.left, rect.bottom - rect.top];
    if size[0] < 32 || size[1] < 32 {
        return Err("Benchmark viewport is too small for a displayed ROI measurement".into());
    }
    Ok(size)
}
impl Drop for Lab {
    fn drop(&mut self) {
        // SAFETY: Synchronized observer teardown; no app/video ownership change.
        unsafe { im_visual_probe_disable() };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipc_replaces_responses_and_refuses_an_offline_arm() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = std::env::temp_dir().join(format!(
            "imirror-visual-lab-{:?}",
            windows::core::GUID::new()?
        ));
        fs::create_dir_all(&directory)?;
        let mut lab = Lab {
            directory: directory.clone(),
            last_id: 0,
            next_poll: Instant::now(),
        };
        fs::write(
            directory.join("request.json"),
            br#"{"id":1,"action":"arm"}"#,
        )?;
        lab.poll(HWND::default(), false)?;
        let first: Value = serde_json::from_slice(&fs::read(directory.join("response.json"))?)?;
        assert_eq!(first["id"], 1);
        assert_eq!(first["ok"], false);
        fs::write(
            directory.join("request.json"),
            br#"{"id":2,"action":"unknown"}"#,
        )?;
        lab.next_poll = Instant::now();
        lab.poll(HWND::default(), false)?;
        let second: Value = serde_json::from_slice(&fs::read(directory.join("response.json"))?)?;
        assert_eq!(second["id"], 2);
        assert_eq!(second["ok"], false);
        assert_eq!(std::mem::size_of::<Observation>(), 40);
        drop(lab);
        fs::remove_file(directory.join("request.json"))?;
        fs::remove_file(directory.join("response.json"))?;
        fs::remove_dir(directory)?;
        Ok(())
    }
}
