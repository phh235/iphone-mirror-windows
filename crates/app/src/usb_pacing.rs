//! Opt-in USB measurement harness. Existing capture/queue/Present policies stay intact.
use crate::{
    benchmark_preview::Preview,
    window_layout::{self, Chrome, Extent},
};
use imirror_native_core::{CaptureOptions, Engine};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::FILETIME,
        System::{
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX},
            Threading::{GetCurrentProcess, GetProcessTimes},
        },
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
    core::w,
};

unsafe extern "C" {
    fn im_frame_pacing_begin() -> i64;
    fn im_frame_pacing_end(path: *const u16) -> i32;
    fn im_frame_pacing_overhead_ns() -> f64;
    fn im_frame_pacing_clock() -> i64;
    fn im_frame_pacing_frequency() -> i64;
}
fn clock() -> i64 {
    // SAFETY: Read-only scalar QPC query; no borrowed pointers or capture state.
    unsafe { im_frame_pacing_clock() }
}
fn cpu_ticks() -> windows::core::Result<u64> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: Current-process pseudo-handle and four valid FILETIME outputs.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )?;
    }
    let ticks = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    Ok(ticks(kernel) + ticks(user))
}
fn memory() -> windows::core::Result<Value> {
    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    // SAFETY: EX is a documented extension with the same initial layout; the
    // valid output buffer length is supplied to GetProcessMemoryInfo.
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            std::mem::size_of_val(&counters) as u32,
        )?;
    }
    Ok(json!({"working_set_bytes":counters.WorkingSetSize,
        "private_bytes":counters.PrivateUsage,"page_fault_count":counters.PageFaultCount}))
}
fn progress(path: &Path, phase: &str, extra: Value) -> std::io::Result<()> {
    fs::write(
        path,
        serde_json::to_vec_pretty(
            &json!({"pid":std::process::id(),"phase":phase,"qpc":clock(),"data":extra}),
        )?,
    )
}
fn finish(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: The recorder is stopped before export; the NUL-terminated path
    // is valid for this synchronous diagnostic-only call.
    if unsafe { im_frame_pacing_end(wide.as_ptr()) } != 0 {
        return Err("Native pacing export failed".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
    };
    if value("--benchmark").map(String::as_str) != Some("usb")
        || !args.iter().any(|a| a == "--render")
    {
        return Err("Frame pacing requires --benchmark usb --render".into());
    }
    let seconds = value("--seconds")
        .map(|v| v.parse::<u64>())
        .transpose()?
        .unwrap_or(60);
    if !(60..=300).contains(&seconds) {
        return Err("Pacing duration must be 60–300 seconds per case".into());
    }
    let cases = value("--cases").map(String::as_str).unwrap_or("ABC");
    if cases.is_empty() || cases.len() > 3 || cases.bytes().any(|c| !b"ABC".contains(&c)) {
        return Err("--cases must select A, B and/or C".into());
    }
    let topmost = args.iter().any(|a| a == "--pacing-topmost");
    let output = Path::new(value("--output").ok_or("Missing --output")?);
    let folder = output.parent().ok_or("Output needs a parent folder")?;
    fs::create_dir_all(folder)?;
    if output.exists()
        || ["A", "B", "C"]
            .iter()
            .any(|c| folder.join(format!("{c}.start")).exists())
    {
        return Err("Use a fresh pacing output directory".into());
    }
    let progress_path = output.with_extension("progress.json");
    let config = crate::settings::load()?; // Read only. Never persist benchmark choices.
    let cpus = std::thread::available_parallelism()?.get();
    // SAFETY: No media threads exist yet; this probes only preallocated event storage.
    let overhead = unsafe { im_frame_pacing_overhead_ns() };
    // SAFETY: Scalar performance-counter frequency.
    let frequency = unsafe { im_frame_pacing_frequency() };
    let engine = Engine::new()?;
    let preview = Preview::new()?;
    // Same child-HWND swap-chain path as the production video, no control backend.
    // SAFETY: Parent outlives the capture session; destroying it later owns the child cleanup.
    let video = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            0,
            0,
            600,
            800,
            Some(preview.0),
            None,
            None,
            None,
        )?
    };
    progress(
        &progress_path,
        "connecting",
        json!({"base_checkpoint":"checkpoint/wda-fast-input-working-20260913","base_commit":"bfa4784a88adf793052b44ad758fa9d7ec7994ba"}),
    )?;
    let devices = engine.devices(true)?;
    let device = devices
        .iter()
        .find(|d| d.connection.eq_ignore_ascii_case("USB"))
        .ok_or("No USB iPhone detected")?;
    let mut options = CaptureOptions::default();
    (options.width, options.height, options.fps) = crate::settings::render_request(config.quality);
    let session = Arc::new(engine.connect(device, options)?);
    // SAFETY: Video child remains alive until the session and its renderer have stopped.
    unsafe {
        session.attach(video.0)?;
        session.view_preferences(video.0, config.one_to_one, config.vsync)?;
    }
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        if !preview.pump() {
            return Err("Benchmark cancelled before warmup".into());
        }
        let status = session.status()?;
        // SAFETY: Read-only counter from the owned live preview.
        if status.state == 4 && unsafe { session.render_submissions(video.0)? } > 0 {
            break;
        }
        if matches!(status.state, 6 | 7) || Instant::now() >= deadline {
            return Err(format!("USB did not reach live video: {}", status.message).into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_secs(10) {
        if !preview.pump() {
            return Err("Benchmark cancelled during warmup".into());
        }
        preview.caption("iMirror USB pacing — warmup, keep phone awake");
        std::thread::sleep(Duration::from_millis(200));
    }
    let format = session.status()?;
    // Reuse the production phone/window sizing policy, with no media modifications.
    // SAFETY: Read DPI and resize only the benchmark's owned parent/child HWNDs.
    let geometry = unsafe {
        let dpi = GetDpiForWindow(preview.0);
        let monitor = window_layout::monitor(preview.0, None)?;
        let chrome = Chrome {
            dpi,
            borders: window_layout::borders(preview.0, dpi)?,
            name: 80,
            status: 80,
            home: false,
        };
        let sizing = chrome
            .natural(
                Extent {
                    width: format.width as i32,
                    height: format.height as i32,
                },
                monitor.rcWork,
            )
            .ok_or("Invalid source dimensions")?;
        let rect = window_layout::centered(sizing.outer, monitor.rcWork);
        SetWindowPos(
            preview.0,
            None,
            rect.left,
            rect.top,
            sizing.outer.width,
            sizing.outer.height,
            SWP_NOZORDER,
        )?;
        MoveWindow(
            video,
            0,
            sizing.toolbar,
            sizing.video.width,
            sizing.video.height,
            true,
        )?;
        if topmost {
            SetWindowPos(
                preview.0,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
        json!({"dpi":dpi,"viewport":[sizing.video.width,sizing.video.height],"toolbar_height":sizing.toolbar,
            "work_area":[monitor.rcWork.left,monitor.rcWork.top,monitor.rcWork.right,monitor.rcWork.bottom],
            "render_path":"D3D11PreviewRenderer, child HWND, existing view preferences"})
    };
    let mut report = json!({"pid":std::process::id(),"base_checkpoint":"checkpoint/wda-fast-input-working-20260913",
        "base_commit":"bfa4784a88adf793052b44ad758fa9d7ec7994ba","instrumented":true,"transport":"USB",
        "phone":{"model":device.model,"ios":device.ios},"source":[format.width,format.height],
        "quality":config.quality,"vsync":config.vsync,"render_cap":options.fps,"view":geometry,
        "logical_processors":cpus,"qpc_frequency":frequency,"recorder_average_hook_ns":overhead,
            "control_backends_started":false,"selected_cases":cases,"benchmark_topmost":topmost,
            "cases":[],"complete":false});
    fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    for (case, description) in [
        ("A", "static"),
        ("B", "continuous scroll"),
        ("C", "fast scroll/animation"),
    ] {
        if !cases.contains(case) {
            continue;
        }
        let gate = folder.join(format!("{case}.start"));
        preview.caption(&format!(
            "iMirror USB pacing — waiting {case}: {description}"
        ));
        progress(&progress_path, &format!("waiting_{case}"), report.clone())?;
        while !gate.exists() {
            if !preview.pump() {
                return Err("Benchmark cancelled while waiting for test".into());
            }
            if matches!(session.status()?.state, 6 | 7) {
                return Err("USB disconnected while waiting".into());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        // SAFETY: This UI serializes recorder begin/end; producers only append observations.
        if unsafe { im_frame_pacing_begin() } == 0 {
            return Err("Recorder allocation failed".into());
        }
        let trace = folder.join(format!("{case}.native.csv"));
        let measured = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let lead = Instant::now();
            while lead.elapsed() < Duration::from_secs(2) {
                if !preview.pump() {
                    return Err("Cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let qpc_start = clock();
            let started = Instant::now();
            let cpu_start = cpu_ticks()?;
            let mut previous = (started.elapsed().as_secs_f64(), cpu_start);
            let mut cpu_samples = Vec::new();
            let mut next_sample = 1.0;
            let mut changed = false;
            let initial = session.status()?;
            while started.elapsed() < Duration::from_secs(seconds) {
                if !preview.pump() {
                    return Err("Benchmark cancelled during measurement".into());
                }
                let status = session.status()?;
                if matches!(status.state, 6 | 7) {
                    return Err(format!("USB stopped: {}", status.message).into());
                }
                changed |= (status.width, status.height) != (initial.width, initial.height);
                let elapsed = started.elapsed().as_secs_f64();
                if elapsed >= next_sample {
                    let cpu = cpu_ticks()?;
                    let dt = elapsed - previous.0;
                    cpu_samples.push(json!({"elapsed":elapsed,"cpu_percent":(cpu-previous.1)as f64/100000.0/dt/cpus as f64,
                        "memory":memory()?,
                        // SAFETY: Read-only queries of the live owned HWND.
                        "window":unsafe {json!({"minimized":IsIconic(preview.0).as_bool(),
                            "visible":IsWindowVisible(preview.0).as_bool(),"foreground":GetForegroundWindow()==preview.0})},
                        "source_frames":status.source_frames,"decoder":session.decoder_mode(),"source":[status.width,status.height]}));
                    previous = (elapsed, cpu);
                    next_sample += 1.0;
                    preview.caption(&format!(
                        "iMirror USB pacing — {case} {description}: {:.0}/{seconds}s",
                        elapsed
                    ));
                    progress(
                        &progress_path,
                        &format!("measuring_{case}"),
                        json!({"elapsed":elapsed,"source_frames":status.source_frames}),
                    )?;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            let qpc_end = clock();
            let elapsed = started.elapsed().as_secs_f64();
            let cpu_end = cpu_ticks()?;
            let tail = Instant::now();
            while tail.elapsed() < Duration::from_secs(2) {
                if !preview.pump() {
                    return Err("Cancelled during tail".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(
                json!({"case":case,"description":description,"qpc_start":qpc_start,"qpc_end":qpc_end,"elapsed_seconds":elapsed,
                "cpu_average_percent":(cpu_end-cpu_start)as f64/100000.0/elapsed/cpus as f64,"cpu_samples":cpu_samples,
                "format_changed":changed,"decoder":session.decoder_mode(),"trace":trace.file_name().map(|s|s.to_string_lossy()),
                "tail_seconds":2,"complete":true}),
            )
        })();
        finish(&trace)?;
        match measured {
            Ok(value) => report["cases"]
                .as_array_mut()
                .ok_or("Invalid report")?
                .push(value),
            Err(error) => {
                report["error"] = json!(error.to_string());
                fs::write(output, serde_json::to_vec_pretty(&report)?)?;
                return Err(error);
            }
        }
        fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    }
    report["complete"] = json!(true);
    fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    progress(&progress_path, "complete", report)?;
    Ok(())
}
