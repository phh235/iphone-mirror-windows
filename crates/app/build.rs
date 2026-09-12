use std::{env, path::PathBuf, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../assets");
    let manifest = assets.join("imirror.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-bin=iMirror=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bin=iMirror=/MANIFESTINPUT:{}",
        manifest.display()
    );
    let resource = assets.join("imirror.rc");
    let icon = assets.join("imirror.ico");
    println!("cargo:rerun-if-changed={}", resource.display());
    println!("cargo:rerun-if-changed={}", icon.display());
    let host = env::var("HOST")?;
    let architecture = host
        .split('-')
        .next()
        .ok_or("Missing build host architecture")?;
    let sdk = find_msvc_tools::find_windows_sdk(architecture)
        .ok_or("Windows SDK is required to embed the application icon")?;
    let compiler = sdk
        .path()
        .map(|p| p.join("rc.exe"))
        .find(|p| p.is_file())
        .ok_or("Windows SDK resource compiler rc.exe was not found")?;
    let output = PathBuf::from(env::var("OUT_DIR")?).join("imirror.res");
    let status = Command::new(compiler)
        .current_dir(&assets)
        .arg("/nologo")
        .arg("/fo")
        .arg(&output)
        .arg(&resource)
        .status()?;
    if !status.success() {
        return Err("Application icon resource compilation failed".into());
    }
    println!("cargo:rustc-link-arg-bin=iMirror={}", output.display());
    Ok(())
}
