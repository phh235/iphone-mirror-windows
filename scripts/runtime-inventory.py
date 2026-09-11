"""Build-time inventory of the exact native runtime and Cargo license notices."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
MSYS = ROOT / "work/dependencies/msys64"

def fields(path):
    result = {}
    key = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("%") and line.endswith("%"):
            key = line.strip("%")
            result[key] = []
        elif key and line:
            result[key].append(line)
    return result

def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runtime", type=Path, default=ROOT / "work/runtime/AirPlay")
    ap.add_argument("--output", type=Path, default=ROOT / "work/notices")
    args = ap.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    packages = {}
    owners = {}
    for folder in (MSYS / "var/lib/pacman/local").iterdir():
        if not (folder / "desc").exists():
            continue
        info = fields(folder / "desc")
        name = info["NAME"][0]
        packages[name] = info
        for item in fields(folder / "files").get("FILES", []):
            if not item.endswith("/"):
                owners[item.lower()] = name
    runtime = []
    used = set()
    mini = ROOT / "work/ffmpeg-audio-install/bin"
    for item in sorted(args.runtime.rglob("*")):
        if item.suffix.lower() not in (".exe", ".dll"):
            continue
        relative = item.relative_to(args.runtime).as_posix()
        if item.name.lower() == "uxplay.exe":
            owner = "UxPlay"
        elif (mini / item.name).exists() and digest(item) == digest(mini / item.name):
            owner = "FFmpeg-minimal-audio"
        else:
            source_relative = "ucrt64/" + (relative if relative.startswith("lib/") else "bin/" + relative)
            owner = owners.get(source_relative.lower())
            if owner is None:
                raise RuntimeError("No native package provenance: " + relative)
            if digest(item) != digest(MSYS / source_relative):
                raise RuntimeError("Runtime differs from package: " + relative)
            used.add(owner)
        runtime.append({"path": relative, "package": owner, "sha256": digest(item), "bytes": item.stat().st_size})
    package_rows = []
    missing_licenses = []
    for name in sorted(used):
        info = packages[name]
        notices = [p for p, owner in owners.items() if owner == name and "/share/licenses/" in p]
        # Runtime subpackages sometimes share the parent package's notices.
        if not notices:
            short = name.removeprefix("mingw-w64-ucrt-x86_64-")
            candidates = [short, short.removesuffix("-libs"), short.removesuffix("-runtime"), "gcc" if short == "cc-libs" else short]
            for candidate in candidates:
                folder = MSYS / "ucrt64/share/licenses" / candidate
                if folder.is_dir():
                    notices.extend(p.relative_to(MSYS).as_posix() for p in folder.rglob("*") if p.is_file())
        if not notices:
            extra = ROOT / "vendor/licenses" / name
            if extra.is_dir():
                shutil.copytree(extra, args.output / "native" / name, dirs_exist_ok=True)
                notices = [str(p) for p in extra.rglob("*") if p.is_file()]
            else:
                missing_licenses.append(name)
        for relative in notices:
            if Path(relative).is_absolute():
                continue
            source = MSYS / relative
            target = args.output / "native" / name / Path(relative).relative_to("ucrt64/share/licenses")
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        package_rows.append({"name": name, "version": info["VERSION"][0], "base": info.get("BASE", [name])[0],
            "license": " OR ".join(info.get("LICENSE", [])), "url": info.get("URL", [""])[0], "notice_files": len(notices)})
    for name, source in [("UxPlay", ROOT/"vendor/uxplay/LICENSE"),
                         ("iPhoneMirror", ROOT/"vendor/iphone-mirror/LICENSE"),
                         ("windows-ble-hid", ROOT/"vendor/licenses/windows-ble-hid-MIT.txt")]:
        target = args.output / "native" / name / source.name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    for folder in [ROOT/"vendor/iphone-mirror/third_party", ROOT/"vendor/uxplay/lib", ROOT/"work/ffmpeg-audio-source"]:
        for source in folder.rglob("*"):
            if source.is_file() and (source.name.upper().startswith(("LICENSE", "COPYING", "NOTICE"))
                or source.name.lower().endswith(".license")):
                relative = source.relative_to(folder)
                target = args.output / "native" / folder.name / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, target)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", "x86_64-pc-windows-msvc"], cwd=ROOT))
    cargo_rows = []
    for package in metadata["packages"]:
        if package["source"] is None:
            continue
        folder = Path(package["manifest_path"]).parent
        notices = [p for p in folder.rglob("*") if p.is_file()
            and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE"))
            and len(p.relative_to(folder).parts) <= 3]
        for source in notices:
            target = args.output / "cargo" / (package["name"] + "-" + package["version"]) / source.relative_to(folder)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        if not notices:
            extra = list((ROOT / "vendor/licenses").glob(package["name"] + "-LICENSE*"))
            for source in extra:
                target = args.output / "cargo" / (package["name"] + "-" + package["version"]) / source.name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, target)
            notices = extra
            if not notices:
                missing_licenses.append(package["name"] + "-" + package["version"])
        cargo_rows.append({"name": package["name"], "version": package["version"],
            "license": package["license"], "repository": package["repository"], "notice_files": len(notices)})
    shutil.copytree(ROOT / "vendor/licenses/wix3", args.output / "native/WiX", dirs_exist_ok=True)
    inventory = {"native_files": runtime, "native_packages": package_rows, "cargo_packages": cargo_rows,
                 "upstream_notice_absent": ["hex-slice-0.1.4: MIT declaration from Cargo.toml; standard text supplied separately"], "missing_notice_files": missing_licenses}
    (args.output/"inventory.json").write_text(json.dumps(inventory, indent=2)+"\n", encoding="utf-8")
    lines = ["# Third-party licenses", "", "iMirror is GPL-3.0-only. This inventory is generated from the staged binaries and Cargo.lock.",
        "Full notices accompany the application in the licenses directory. Dynamic LGPL libraries remain replaceable.",
        "No Apple software, signing credentials, Python runtime, or Node.js runtime is bundled.", "",
        "## Native media", "", "| Component | Version | License |", "|---|---|---|",
        "| iPhoneMirror native core | 2635a0073c67539aec86487844fc41a5caddf9ba | GPL-3.0-only |",
        "| UxPlay | f2c4a66e704859e791e139db4f3fdba79cd838f9 | GPL-3.0-or-later |",
        "| FFmpeg AAC/ALAC libraries | 9.0.1, minimal audio build | LGPL-2.1-or-later |",
        "| windows-ble-hid report design | 9a4f45129779ec3bef368ea4ffcdcde795a57b65 | MIT |",
        "| WiX bootstrapper | 3.14.1 | MS-RL; source and notices supplied separately from the application |",
        "| libusb / libusb-win32 | Pinned native-core third_party files | LGPL-2.1-or-later / LGPL-3.0-only |"]
    lines += [f"| {p['name']} | {p['version']} | {p['license']} |" for p in package_rows]
    lines += ["", "## Rust dependencies", "", "| Crate | Version | License expression |", "|---|---|---|"]
    lines += [f"| {p['name']} | {p['version']} | {p['license']} |" for p in cargo_rows]
    lines += ["", "The exact binary hashes, provenance and notice counts are in licenses/inventory.json.",
        "Corresponding-source packaging and license review remain release gates until documented as complete.", ""]
    if missing_licenses:
        lines += ["Missing notice files (must be resolved before redistribution): " + ", ".join(missing_licenses)]
    (args.output/"THIRD_PARTY_LICENSES.md").write_text("\n".join(lines), encoding="utf-8")
    print(json.dumps({"native_binaries": len(runtime), "native_packages": len(package_rows),
        "cargo_packages": len(cargo_rows), "missing_notices": missing_licenses}, indent=2))

if __name__ == "__main__":
    main()
