use std::{env, path::PathBuf};
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../assets/imirror.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-bin=iMirror=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bin=iMirror=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
