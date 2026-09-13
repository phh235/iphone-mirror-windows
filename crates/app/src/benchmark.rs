use imirror_native_core::{CaptureOptions, Engine};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Capture/decoder benchmark with an optional native preview. Full samples are
/// streamed to disk; the in-memory tail stays bounded during long stress runs.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|a| a == "--frame-pacing") {
        return crate::usb_pacing::run(args);
    }
    let index = args
        .iter()
        .position(|s| s == "--benchmark")
        .ok_or("Missing benchmark mode")?;
    let mode = args
        .get(index + 1)
        .ok_or("--benchmark requires usb or airplay")?;
    if mode != "usb" && mode != "airplay" {
        return Err("--benchmark requires usb or airplay".into());
    }
    let duration = if let Some(i) = args.iter().position(|s| s == "--seconds") {
        args.get(i + 1)
            .ok_or("--seconds requires a value")?
            .parse::<u64>()?
    } else {
        60
    };
    if !(1..=3600).contains(&duration) {
        return Err("Benchmark duration must be 1–3600 seconds".into());
    }
    let index = args
        .iter()
        .position(|s| s == "--output")
        .ok_or("Benchmark requires --output <file.json>")?;
    let output = Path::new(args.get(index + 1).ok_or("--output requires a path")?);
    let sample_path = output.with_extension("samples.jsonl");
    let progress_path = output.with_extension("progress.json");
    let mut log = BufWriter::new(File::create(&sample_path)?);
    let mut report = json!({"transport":mode,"requested_duration_seconds":duration,
        "test":"capture and decode","phone":Value::Null,"source_frames":0,
        "source_fps_measured":Value::Null,"render_fps_measured":Value::Null,
        "end_to_end_latency_ms":Value::Null,"presentation_tested":false,"input_tested":false,
        "samples_file":sample_path.file_name().map(|s|s.to_string_lossy()),"retained_samples_max":300});
    let mut samples = VecDeque::with_capacity(300);
    let mut sample_count = 0u64;
    let mut elapsed = 0.0;
    let mut source_frames = 0;
    let mut rendered = 0;
    let mut baseline: Option<(f64, u64, u64)> = None;
    let result: Result<(), Box<dyn std::error::Error>> = (|| {
        let engine = Engine::new()?;
        // Declaration order preserves the HWND until the receiver and session stop.
        let preview = if args.iter().any(|s| s == "--render") {
            Some(crate::benchmark_preview::Preview::new()?)
        } else {
            None
        };
        let mut receiver = None;
        let session = if mode == "usb" {
            let devices = engine.devices(true)?;
            let device = devices.first().ok_or("No iPhone is connected")?;
            report["phone"] = json!({"model":device.model,"ios":device.ios});
            Arc::new(engine.connect(device, CaptureOptions::default())?)
        } else {
            let session = Arc::new(engine.encoded(CaptureOptions::default())?);
            let exe = std::env::current_exe()?;
            let runtime = exe
                .parent()
                .ok_or("Executable directory is unavailable")?
                .join("AirPlay");
            receiver = Some(imirror_video_airplay::Receiver::start(
                session.clone(),
                &runtime,
                "iMirror benchmark",
                1920,
                1080,
                60,
            )?);
            session
        };
        if let Some(preview) = &preview {
            // SAFETY: Preview outlives the owned session and its renderer.
            unsafe {
                session.attach(preview.0.0)?;
            }
        }
        report["renderer_attached"] = json!(preview.is_some());
        let start = Instant::now();
        let mut saw_decoded = false;
        while start.elapsed() < Duration::from_secs(duration) {
            if let Some(preview) = &preview
                && !preview.pump()
            {
                return Err("Benchmark cancelled before its requested duration".into());
            }
            std::thread::sleep(Duration::from_millis(200));
            elapsed = start.elapsed().as_secs_f64();
            let status = session.status()?;
            if let Some(preview) = &preview {
                // SAFETY: Preview belongs to the live owned session.
                rendered = unsafe { session.render_submissions(preview.0.0)? };
            }
            if status.source_frames < source_frames {
                return Err(
                    "Source frame counter moved backwards during the same capture session".into(),
                );
            }
            source_frames = status.source_frames;
            if source_frames > 0 && baseline.is_none() {
                baseline = Some((elapsed, source_frames, rendered));
            }
            let decoder = session.decoder_mode();
            saw_decoded |= status.state == 4 && decoder != "unknown";
            let sample = json!({"elapsed_seconds":elapsed,"status":status,"decoder":decoder,"render_submitted_frames":rendered});
            serde_json::to_writer(&mut log, &sample)?;
            log.write_all(b"\n")?;
            sample_count += 1;
            if samples.len() == 300 {
                samples.pop_front();
            }
            samples.push_back(sample.clone());
            if sample_count.is_multiple_of(5) {
                log.flush()?;
                std::fs::write(&progress_path, serde_json::to_vec_pretty(&sample)?)?;
                if let Some(preview) = &preview {
                    preview.caption(&format!(
                        "iMirror - USB validation | {} x {} | source {:.1} FPS | {}",
                        status.width,
                        status.height,
                        status.source_fps.unwrap_or(0.0),
                        decoder
                    ));
                }
            }
            if matches!(status.state, 6 | 7) {
                return Err(status.message.clone().into());
            }
            if let Some(receiver) = &mut receiver
                && let Some(error) = receiver.failure()
            {
                return Err(error.into());
            }
        }
        if let Some(receiver) = &mut receiver
            && !receiver.stop()
        {
            return Err("Receiver required forced shutdown".into());
        }
        if source_frames == 0 {
            return Err("No source frames were received during the benchmark".into());
        }
        if !saw_decoded {
            return Err("Encoded frames arrived but no decoded frame was confirmed".into());
        }
        if preview.is_some() && rendered == 0 {
            return Err("No frame was submitted by the native renderer".into());
        }
        Ok(())
    })();
    log.flush()?;
    report["elapsed_seconds"] = json!(elapsed);
    report["source_frames"] = json!(source_frames);
    report["render_submitted_frames"] = json!(rendered);
    report["sample_count"] = json!(sample_count);
    report["samples"] = json!(samples);
    if let Some((at, frames, submitted)) = baseline
        && elapsed - at > 0.5
    {
        report["source_fps_measured"] =
            json!(source_frames.saturating_sub(frames) as f64 / (elapsed - at));
        if report["renderer_attached"] == true {
            report["render_fps_measured"] =
                json!(rendered.saturating_sub(submitted) as f64 / (elapsed - at));
        }
    }
    report["passed_capture_check"] = json!(result.is_ok());
    if let Err(error) = &result {
        report["error"] = json!(error.to_string());
    }
    std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    result
}
