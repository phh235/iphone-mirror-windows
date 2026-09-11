use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../vendor/iphone-mirror");
    let core = root.join("src/Core");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .static_crt(true)
        .flag("/EHsc")
        .flag("/utf-8")
        .flag("/bigobj")
        .define("IPHONEMIRROR_CORE_EXPORTS", None)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("NOMINMAX", None)
        .include(core.join("include"))
        .include(core.join("src"))
        .include(root.join("src/WirelessHost"))
        .include(root.join("third_party/libusb/include"))
        .include(root.join("third_party/libusb-win32/include"));
    for name in [
        "CoreApi",
        "Logging",
        "Protocol/Plist",
        "Protocol/QuickTimePacket",
        "Protocol/QuickTimeSession",
        "Transport/Socket",
        "Transport/UsbMuxClient",
        "Transport/QtUsbTransport",
        "Transport/LibUsb0Transport",
        "Device/AppleUsbDiscovery",
        "Device/DeviceManager",
        "Capture/CaptureSession",
        "Capture/EncodedSession",
        "Capture/WirelessReceiverHub",
        "Capture/WirelessCaptureSession",
        "Media/H264",
        "Media/CoreMedia",
        "Media/MediaFoundationDecoder",
        "Audio/WasapiRenderer",
        "Renderer/D3D11PreviewRenderer",
    ] {
        build.file(core.join("src").join(format!("{name}.cpp")));
    }
    build.compile("imirror_native");
    for library in [
        "ws2_32",
        "bcrypt",
        "setupapi",
        "cfgmgr32",
        "advapi32",
        "mfplat",
        "mf",
        "mfuuid",
        "wmcodecdspuuid",
        "d3d11",
        "d3dcompiler",
        "dcomp",
        "dxgi",
        "ole32",
        "oleaut32",
        "avrt",
        "winhttp",
    ] {
        println!("cargo:rustc-link-lib={library}");
    }
    for (directory, name) in [
        ("libusb", "libusb-1.0"),
        ("libusb-win32", "libusb0-dynamic"),
    ] {
        println!(
            "cargo:rustc-link-search=native={}",
            root.join(format!("third_party/{directory}/lib/x64"))
                .display()
        );
        println!("cargo:rustc-link-lib={name}");
    }
    // Cargo places test executables in deps and applications one level above it.
    let out = PathBuf::from(env::var("OUT_DIR")?);
    let profile = out
        .ancestors()
        .nth(3)
        .ok_or("Unexpected Cargo OUT_DIR layout")?;
    fs::create_dir_all(profile.join("deps"))?;
    for (directory, name) in [
        ("libusb", "libusb-1.0.dll"),
        ("libusb-win32", "libusb0.dll"),
    ] {
        let source = root.join(format!("third_party/{directory}/bin/x64/{name}"));
        fs::copy(&source, profile.join(name))?;
        fs::copy(&source, profile.join("deps").join(name))?;
    }

    // The USB transport launches this bounded helper beside the application.
    // Build it from the pinned source for both cargo run and packaged releases.
    let helper_name = "iPhoneMirror.UsbConfigurationSwitch.exe";
    let helper_path = profile.join(helper_name);
    let mut helper = build.get_compiler().to_command();
    helper
        .arg("/nologo")
        .arg(core.join("tools/UsbConfigurationSwitch.cpp"))
        .arg(format!(
            "/Fo{}",
            out.join("UsbConfigurationSwitch.obj").display()
        ))
        .arg(format!("/Fe{}", helper_path.display()))
        .arg("/link")
        .arg("cfgmgr32.lib")
        .arg(root.join("third_party/libusb-win32/lib/x64/libusb0-dynamic.lib"));
    let result = helper.status()?;
    if !result.success() {
        return Err(format!("USB configuration helper build failed: {result}").into());
    }
    fs::copy(&helper_path, profile.join("deps").join(helper_name))?;
    println!("cargo:rerun-if-changed={}", root.display());
    Ok(())
}
