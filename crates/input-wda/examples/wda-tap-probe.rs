//! Real, opt-in Calculator fixture. Does not capture Windows input or automate setup.
use imirror_input_core::Controller;
use imirror_input_wda::{
    Wda,
    probe::{Node, nodes},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn calculator(client: &mut Wda) -> Result<(), Box<dyn std::error::Error>> {
    if client.probe_active_app()?["value"]["bundleId"] != "com.apple.calculator" {
        return Err("Calculator is not foreground; no input sent".into());
    }
    Ok(())
}
fn result_is(tree: &[Node], band: &Node, expected: &str) -> bool {
    tree.iter().any(|n| {
        !n.button()
            && (!band.result_display() || n.result_display())
            && n.has(expected)
            && n.height >= band.height * 0.5
            && (n.y - band.y).abs() < band.height.max(20.0) * 0.6
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let output = PathBuf::from(option("--output").ok_or("--output required")?);
    let setup_only = args.iter().any(|a| a == "--inspect-only");
    if !setup_only && !args.iter().any(|a| a == "--confirm-calculator-test") {
        return Err("Explicit --confirm-calculator-test required for real input".into());
    }
    #[cfg(windows)]
    let _awake = (!setup_only)
        .then(imirror_input_wda::probe_visual::KeepAwake::new)
        .transpose()?;
    if let Some(gate) = option("--start-gate") {
        let gate = PathBuf::from(gate);
        if !gate.is_absolute() || gate.exists() {
            return Err("Use a fresh absolute start-gate file".into());
        }
        println!("Waiting for the local start gate; idle sleep inhibited, no phone input sent.");
        let deadline = Instant::now() + Duration::from_secs(600);
        while !gate.is_file() {
            if Instant::now() >= deadline || gate.with_extension("stop").exists() {
                return Err("Start gate cancelled or timed out; no phone input sent".into());
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    if output.exists() {
        return Err("Use a fresh output directory".into());
    }
    fs::create_dir_all(&output)?;
    let count = option("--samples")
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(100);
    if args.iter().any(|a| a == "--adopt-running-session")
        && args.iter().any(|a| a == "--manage-runtime")
    {
        return Err("Do not combine runtime ownership with session adoption".into());
    }
    if !(100..=200).contains(&count) {
        return Err("Use 100–200 samples per contact duration".into());
    }
    #[cfg(windows)]
    let _runtime = if args.iter().any(|a| a == "--manage-runtime") {
        let runtime = imirror_input_wda::runtime::ManagedRuntime::start()?;
        let deadline = Instant::now() + Duration::from_secs(90);
        while !runtime.status().ready {
            if Instant::now() >= deadline {
                return Err("Managed WDA did not become ready".into());
            }
            thread::sleep(Duration::from_millis(250));
        }
        Some(runtime)
    } else {
        None
    };
    let mut client = Wda::new("http://127.0.0.1:8100")?;
    if args.iter().any(|a| a == "--adopt-running-session") {
        if args.iter().any(|a| a == "--manage-runtime") {
            return Err("Do not combine runtime ownership with session adoption".into());
        }
        client.probe_adopt_running_session()?;
    }
    let size = client.geometry()?;
    calculator(&mut client)?;
    let initial = nodes(&client.probe_source()?)?;
    let ones: Vec<_> = initial
        .iter()
        .filter(|n| n.button() && n.has("1"))
        .collect();
    if ones.len() != 1 {
        return Err("Cannot identify exactly one Calculator key 1; no input sent".into());
    }
    let one = ones[0].clone();
    let clears: Vec<_> = initial
        .iter()
        .filter(|n| {
            n.button()
                && n.y < one.y
                && n.y > size.height * 0.25
                && n.texts.iter().any(|s| {
                    matches!(
                        s.to_lowercase().as_str(),
                        "ac" | "c"
                            | "all clear"
                            | "clear"
                            | "xóa"
                            | "xoá"
                            | "xóa tất cả"
                            | "xoá tất cả"
                    )
                })
        })
        .collect();
    if clears.len() != 1 {
        return Err("Cannot identify exactly one clear key; no input sent".into());
    }
    let clear = clears[0].clone();
    let canonical_result = initial.iter().any(Node::result_display);
    let reset_tree = if !setup_only
        && !initial.iter().any(|n| {
            !n.button()
                && (!canonical_result || n.result_display())
                && n.has("0")
                && n.height >= 24.
                && n.y < clear.y
        }) {
        calculator(&mut client)?;
        client.probe_tap(clear.center(), 50)?;
        fs::write(
            output.join("preparation.json"),
            br#"{"clear_sent":true,"measured_taps_sent":0}"#,
        )?;
        thread::sleep(Duration::from_millis(200));
        Some(nodes(&client.probe_source()?)?)
    } else {
        None
    };
    let zeros: Vec<_> = reset_tree
        .as_deref()
        .unwrap_or(&initial)
        .iter()
        .filter(|n| {
            !n.button()
                && (!canonical_result || n.result_display())
                && n.has("0")
                && n.height >= 24.
                && n.y < clear.y
        })
        .collect();
    if zeros.len() != 1 {
        let details:Vec<_>=reset_tree.as_deref().unwrap_or(&initial).iter()
            .filter(|n|n.texts.iter().any(|t|t.starts_with("StandardInputView")))
            .map(|n|json!({"kind":n.kind,"texts":n.texts,"y":n.y,"height":n.height,"matches_zero":n.has("0")})).collect();
        fs::write(
            output.join("fixture-error.json"),
            serde_json::to_vec_pretty(&json!({
            "zero_candidates":zeros.len(),"clear_y":clear.y,"result_nodes":details}))?,
        )?;
        return Err(
            "Calculator display must show exactly one identifiable 0; no measured taps sent".into(),
        );
    }
    let display = zeros[0].clone();
    let mut report = json!({"fixture":"physical Calculator, clear then 1","generated_input":true,
        "windows_mouse_timestamps":false,"samples_per_contact":count,
        "idle_sleep_inhibited":cfg!(windows),
        "contacts_ms":[50,30,20,10],"device_points":[size.width,size.height],
        "results":[],"status_probes":[],"complete":false,
        "input_to_visual_ms":null,
        "notes":["Outcome verified from real WDA accessibility state after HTTP completion; not inferred from HTTP success.",
            "This fixture excludes Windows UI queue. Optional ROI observations require ETW correlation for first displayed response.",
            "Status requests measure HTTP/forwarding plus their own handler; never subtract as exact XCTest time.",
            "Contact order is rotated each round; no input retries after ambiguous failures."]});
    #[cfg(windows)]
    let mut visual = option("--visual-lab")
        .map(|p| imirror_input_wda::probe_visual::Visual::new(PathBuf::from(p)))
        .transpose()?;
    #[cfg(windows)]
    if let Some(lab) = &mut visual {
        // The real Calculator result is right-aligned in a wide container.
        // Concentrate the fixed grid on the single-digit 0/1 fixture, excluding
        // the blank left area and the keypad below the result container.
        let left = (display.x + (display.width - 70.0).max(0.0)) / size.width;
        let top = display.y / size.height;
        let right = (display.x + display.width).min(size.width) / size.width;
        let bottom = (display.y + display.height).min(size.height) / size.height;
        report["visual_config"] = lab.configure(left, top, right - left, bottom - top)?;
        if !setup_only {
            let zero_keys: Vec<_> = initial
                .iter()
                .filter(|n| n.button() && n.has("0"))
                .collect();
            if zero_keys.len() != 1 {
                return Err("Cannot identify preparation key 0".into());
            }
            calculator(&mut client)?;
            client.probe_tap(zero_keys[0].center(), 50)?;
            report["preparation_zero_tap"] = json!(true);
            thread::sleep(Duration::from_millis(200));
            if !result_is(&nodes(&client.probe_source()?)?, &display, "0") {
                return Err(
                    "Preparation did not leave Calculator at 0; no measured taps sent".into(),
                );
            }
        }
        let mut noise = 0;
        for _ in 0..3 {
            let arm = lab.arm()?;
            thread::sleep(Duration::from_millis(200));
            let observation = lab.read(
                arm["generation"]
                    .as_u64()
                    .ok_or("Invalid visual generation")?,
            )?;
            noise += observation["events"].as_array().map_or(0, Vec::len);
        }
        report["no_input_changed_frames"] = json!(noise);
        if noise != 0 {
            fs::write(
                output.join("results.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            return Err("ROI changed without input; do not claim visual latency".into());
        }
    }
    if setup_only {
        report["inspection_complete"] = json!(true);
        fs::write(
            output.join("results.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("Calculator fixture identified; no input sent.");
        return Ok(());
    }
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..30 {
            client.status()?;
            let timing = client.last_timing.ok_or("No HTTP timing")?;
            report["status_probes"]
                .as_array_mut()
                .ok_or("Invalid report")?
                .push(timing.json());
            thread::sleep(Duration::from_millis(50));
        }
        let contacts = [50u64, 30, 20, 10];
        for round in 0..count {
            for slot in 0..contacts.len() {
                if output.join("STOP").exists() {
                    return Err("Stopped by user before next gesture".into());
                }
                let contact = contacts[(round + slot) % contacts.len()];
                calculator(&mut client)?;
                client.probe_tap(clear.center(), 50)?;
                thread::sleep(Duration::from_millis(100));
                let before = nodes(&client.probe_source()?)?;
                if !result_is(&before, &display, "0") {
                    return Err("Clear did not yield 0; stop fixture".into());
                }
                calculator(&mut client)?;
                #[cfg(windows)]
                let visual_arm = visual.as_mut().map(|v| v.arm()).transpose()?;
                #[cfg(windows)]
                let input_qpc = imirror_input_wda::probe_visual::qpc()?;
                let started = Instant::now();
                let dispatched = client.probe_tap(one.center(), contact);
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.;
                let timing = client.last_timing;
                let mut record = json!({"round":round,"contact_ms":contact,"http_success":dispatched.is_ok(),
                    "dispatch_ms":elapsed_ms,"http":timing.map(|t|t.json()),"outcome":"UNVERIFIED"});
                #[cfg(windows)]
                {
                    record["generated_input_qpc"] = json!(input_qpc);
                    record["visual_arm"] = json!(visual_arm);
                }
                if let Err(error) = dispatched {
                    record["outcome"] = json!("UNKNOWN_HTTP_FAILURE");
                    report["results"]
                        .as_array_mut()
                        .ok_or("Invalid report")?
                        .push(record);
                    return Err(error.into());
                }
                // Verification is after the measured HTTP request, not concurrent
                // screenshot polling that could compete with XCTest execution.
                let verification = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let after = nodes(&client.probe_source()?)?;
                    record["outcome"] = json!(if result_is(&after, &display, "1") {
                        "PASS"
                    } else if result_is(&after, &display, "0") {
                        "MISSED"
                    } else {
                        "WRONG_OR_UNRECOGNIZED"
                    });
                    #[cfg(windows)]
                    if let Some(lab) = &mut visual
                        && let Some(arm) = &visual_arm
                    {
                        record["visual"] = lab.finish(
                            arm["generation"]
                                .as_u64()
                                .ok_or("Invalid visual generation")?,
                        )?;
                    }
                    Ok(())
                })();
                if let Err(error) = &verification {
                    record["verification_error"] = json!(error.to_string());
                }
                report["results"]
                    .as_array_mut()
                    .ok_or("Invalid report")?
                    .push(record);
                verification?;
                thread::sleep(Duration::from_millis(200));
            }
            fs::write(
                output.join("results.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            println!("Completed round {}/{}", round + 1, count);
        }
        Ok(())
    })();
    report["complete"] = json!(result.is_ok());
    #[cfg(windows)]
    if let Some(lab) = &mut visual {
        let _ = lab.disable();
    }
    if let Err(error) = &result {
        report["error"] = json!(error.to_string());
    }
    let records = report["results"].as_array().ok_or("Invalid report")?;
    let summaries:Vec<Value>=[50,30,20,10].iter().map(|contact|{
        let selected:Vec<_>=records.iter().filter(|r|r["contact_ms"].as_i64()==Some(*contact)).collect();
        let mut times:Vec<_>=selected.iter().filter_map(|r|r["dispatch_ms"].as_f64()).collect();
        times.sort_by(f64::total_cmp);
        let p=|percent:usize|(!times.is_empty()).then(||times[(times.len()*percent).div_ceil(100)-1]);
        json!({"contact_ms":contact,"attempts":selected.len(),
            "verified_pass":selected.iter().filter(|r|r["outcome"]=="PASS").count(),
            "missed":selected.iter().filter(|r|r["outcome"]=="MISSED").count(),
            "wrong_or_unrecognized":selected.iter().filter(|r|r["outcome"]=="WRONG_OR_UNRECOGNIZED").count(),
            "p50_ms":p(50),"p95_ms":p(95),"p99_ms":p(99)})
    }).collect();
    report["summary"] = json!(summaries);
    fs::write(
        output.join("results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report["summary"])?);
    result
}
