"""Assemble corresponding source, build recipes and exact dependency archives."""
import concurrent.futures,json,subprocess,zipfile
from pathlib import Path
import importlib.util
_helper_path = Path(__file__).resolve().parent / "build-airplay.py"
_spec = importlib.util.spec_from_file_location("build_airplay",_helper_path)
_helper = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_helper)
fetch,hash_file = _helper.fetch,_helper.hash_file
ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT/"work"
def main():
    native=WORK/"corresponding-source/native"
    lock=json.loads((ROOT/"docs/native-source-lock.json").read_text())
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(lambda row:fetch(row,native),lock))
    cargo=WORK/"corresponding-source/cargo"
    with (WORK/"cargo-vendor-config.txt").open("w") as config:
        subprocess.run(["cargo","vendor","--locked","--versioned-dirs",str(cargo)],cwd=ROOT,stdout=config,check=True)
    output=ROOT/"dist/iMirror-0.1.0-source.zip"
    output.parent.mkdir(parents=True,exist_ok=True)
    roots=["Cargo.toml","Cargo.lock","LICENSE","README.md","HUONG_DAN.md",".github",".gitignore","crates","scripts","installer","docs","vendor","assets"]
    with zipfile.ZipFile(output,"w",compression=zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as z:
        for name in roots:
            path=ROOT/name
            files=path.rglob("*") if path.is_dir() else [path]
            for file in files:
                if file.is_file() and "__pycache__" not in file.parts:
                    z.write(file,"iMirror/"+file.relative_to(ROOT).as_posix())
        config=(ROOT/".cargo/config.toml").read_text()
        config+='\n[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "dependencies/cargo"\n'
        z.writestr("iMirror/.cargo/config.toml",config)
        for file in cargo.rglob("*"):
            if file.is_file():z.write(file,"iMirror/dependencies/cargo/"+file.relative_to(cargo).as_posix())
        for row in lock:
            file=native/row["file"]
            if hash_file(file)!=row["sha256"]:raise RuntimeError("Source archive hash mismatch")
            z.write(file,"iMirror/work/corresponding-source/native/"+file.name,compress_type=zipfile.ZIP_STORED)
        z.writestr("iMirror/SOURCE_BUILD.txt",
            "This archive contains the application, native patches, every Cargo.lock dependency, and the exact native library source packages.\n"
            "Run powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1 on a Windows x64 build host.\n"
            "Rust MSVC, VS C++ Build Tools, CMake and Python are build-time prerequisites. The pinned MSYS and WiX build tool archives download on first use.\n"
            "Native .src.tar.zst files include the original upstream tarballs and MSYS2 PKGBUILD recipes; do not omit them when redistributing this source archive.\n")
    print(f"Source archive: {output} ({output.stat().st_size/1048576:.1f} MiB)")
if __name__=="__main__":main()
