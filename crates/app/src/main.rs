#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_icon;
mod app_instance;
mod benchmark;
mod benchmark_preview;
mod ble_panel;
mod control;
mod diagnostics;
mod i18n;
mod low_latency;
mod pointer_settings;
mod raw_input;
mod settings;
mod settings_window;
mod svg_icons;
mod theme;
mod toolbar;
mod ui;
mod ui_input;
mod ui_snapshot;
mod usb_pacing;
mod wda_input_trace;
mod window_layout;
mod worker;
use imirror_native_core::Engine;
fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--ui-icon-benchmark") {
        svg_icons::benchmark()?;
    } else if args.iter().any(|a| a == "--benchmark") {
        benchmark::run(&args)?;
    } else if args.iter().any(|a| a == "--version") {
        println!("iMirror {}", env!("CARGO_PKG_VERSION"));
    } else if args.iter().any(|a| a == "--list-devices") {
        let engine = Engine::new()?;
        println!("{}", serde_json::to_string_pretty(&engine.devices(true)?)?);
    } else if args.iter().any(|a| a == "--airplay-smoke-test") {
        let engine = Engine::new()?;
        let session = std::sync::Arc::new(engine.encoded(Default::default())?);
        let executable = std::env::current_exe()?;
        let runtime = executable
            .parent()
            .ok_or("No executable directory")?
            .join("AirPlay");
        let mut receiver = imirror_video_airplay::Receiver::start(
            session,
            &runtime,
            "iMirror validation",
            1920,
            1080,
            60,
        )?;
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(error) = receiver.failure() {
                return Err(error.into());
            }
        }
        if !receiver.stop() {
            return Err("AirPlay receiver did not shut down gracefully".into());
        }
        println!("AirPlay process startup and shutdown passed; no phone stream was tested.");
    } else if args.iter().any(|a| a == "--diagnostics") {
        let host = imirror_platform_windows::detect()?;
        let json = serde_json::to_string_pretty(&host)?;
        if let Some(index) = args.iter().position(|a| a == "--output") {
            let path = args.get(index + 1).ok_or("--output requires a path")?;
            std::fs::write(path, &json)?;
        }
        println!("{json}");
    } else {
        let smoke = args.iter().any(|a| a == "--ui-smoke-test");
        let _instance = if smoke {
            None
        } else {
            match app_instance::Instance::acquire()? {
                Some(instance) => Some(instance),
                None => {
                    app_instance::show_existing();
                    return Ok(());
                }
            }
        };
        ui::run(smoke)?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = execute() {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            eprintln!("iMirror: {error}");
            std::process::exit(1);
        }
        let message: Vec<u16> = format!("{}:\n{error}", i18n::tr("iMirror could not start"))
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: Message is NUL-terminated and remains live until MessageBox returns.
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                None,
                windows::core::PCWSTR(message.as_ptr()),
                windows::core::w!("iMirror"),
                windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}
