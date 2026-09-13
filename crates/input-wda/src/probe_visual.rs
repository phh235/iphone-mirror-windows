//! Local-file benchmark observations. No remote control service is introduced.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::Win32::System::Performance::QueryPerformanceCounter;

/// Benchmark-only idle sleep/display inhibition. Does not change a power plan
/// or prevent explicit user sleep/lock. Must be dropped on its creating thread.
pub struct KeepAwake {
    previous: windows::Win32::System::Power::EXECUTION_STATE,
    _thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl KeepAwake {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use windows::Win32::System::Power::*;
        // SAFETY: Scalar flags apply only to this fixture thread. The !Send
        // guard restores its previous state on the same thread at scope exit.
        let previous = unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)
        };
        if previous.0 == 0 {
            return Err("Could not inhibit idle sleep for the measurement".into());
        }
        Ok(Self {
            previous,
            _thread_bound: std::marker::PhantomData,
        })
    }
}
impl Drop for KeepAwake {
    fn drop(&mut self) {
        // SAFETY: !Send/!Sync ensures this is the creating thread; restore only
        // that thread's execution requirements, never global power settings.
        unsafe {
            windows::Win32::System::Power::SetThreadExecutionState(self.previous);
        }
    }
}

pub fn qpc() -> windows::core::Result<i64> {
    let mut value = 0;
    // SAFETY: Valid scalar output; QPC is shared across processes on this host.
    unsafe {
        QueryPerformanceCounter(&mut value)?;
    }
    Ok(value)
}
pub struct Visual {
    directory: PathBuf,
    next_id: u64,
}
impl Visual {
    pub fn new(directory: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if !directory.is_absolute() || !directory.is_dir() {
            return Err("Existing absolute visual-lab directory required".into());
        }
        Ok(Self {
            directory,
            next_id: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
        })
    }
    fn request(&mut self, mut body: Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.next_id = self.next_id.saturating_add(1);
        body["id"] = json!(self.next_id);
        let temporary = self.directory.join("request.tmp");
        fs::write(&temporary, serde_json::to_vec(&body)?)?;
        fs::rename(temporary, self.directory.join("request.json"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.directory.join("STOP").exists() {
                return Err("Visual benchmark cancelled".into());
            }
            if let Ok(bytes) = fs::read(self.directory.join("response.json")) {
                if bytes.len() > 128 * 1024 {
                    return Err("Oversized visual response".into());
                }
                if let Ok(reply) = serde_json::from_slice::<Value>(&bytes)
                    && reply["id"].as_u64() == Some(self.next_id)
                {
                    if reply["ok"] != true {
                        return Err(reply["error"]
                            .as_str()
                            .unwrap_or("Visual lab error")
                            .to_owned()
                            .into());
                    }
                    return Ok(reply["value"].clone());
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err("Visual lab response timeout".into())
    }
    pub fn configure(
        &mut self,
        left: f64,
        top: f64,
        width: f64,
        height: f64,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        self.request(
            json!({"action":"configure","left":left,"top":top,"width":width,"height":height}),
        )
    }
    pub fn arm(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.request(json!({"action":"arm"})) {
                Ok(value) => return Ok(value),
                Err(error)
                    if error.to_string().contains("baseline") && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => return Err(error),
            }
        }
    }
    pub fn read(&mut self, generation: u64) -> Result<Value, Box<dyn std::error::Error>> {
        self.request(json!({"action":"read","generation":generation}))
    }
    pub fn finish(&mut self, generation: u64) -> Result<Value, Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let result = self.read(generation)?;
            if result["events"].as_array().is_some_and(|e| !e.is_empty())
                || Instant::now() >= deadline
            {
                return Ok(result);
            }
            thread::sleep(Duration::from_millis(30));
        }
    }
    pub fn disable(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.request(json!({"action":"disable"}))?;
        Ok(())
    }
}
