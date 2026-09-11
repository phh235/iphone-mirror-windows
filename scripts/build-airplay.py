"""Rebuild UxPlay and the small AAC/ALAC runtime using hash-pinned local build tools."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / "work"
DEPS = WORK / "dependencies"
MSYS = DEPS / "msys64"

def run(args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)

def hash_file(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f,"sha256").hexdigest()

def fetch(entry, folder):
    path = folder / entry["file"]
    folder.mkdir(parents=True, exist_ok=True)
    if path.exists() and hash_file(path) == entry["sha256"]:
        return path
    temporary = path.with_name(path.name + ".download")
    run(["curl.exe","--fail","--location","--silent","--show-error","--retry","2","--output",temporary,entry["url"]])
    if hash_file(temporary) != entry["sha256"]:
        raise RuntimeError("Hash mismatch: " + entry["file"])
    temporary.replace(path)
    return path

def bootstrap():
    lock_path = ROOT / "docs/msys-build-lock.json"
    lock = json.loads(lock_path.read_text())
    if lock["missing"]:
        raise RuntimeError("Incomplete build environment lock")
    base = fetch(lock["base"],DEPS)
    if not (MSYS/"usr/bin/bash.exe").exists():
        # This exact base archive is hash verified and contains only the MSYS tree.
        run(["tar.exe","-xf",base,"-C",DEPS])
    cache = MSYS/"var/cache/pacman/pkg"
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        paths = list(pool.map(lambda p:fetch(p,cache),lock["packages"]))
    current = MSYS/"var/lib/pacman/local"
    needed = [p for p in lock["packages"] if not (current/(p["name"]+"-"+p["version"])).is_dir()]
    if needed:
        script = WORK/"install-pinned-msys.sh"
        script.write_text("set -e\npacman -U --noconfirm --needed " +
            " ".join(shlex.quote("/var/cache/pacman/pkg/"+p["file"]) for p in needed) + "\n")
        env = os.environ.copy()
        env.update({"MSYSTEM":"UCRT64","CHERE_INVOKING":"1"})
        bash = MSYS/"usr/bin/bash.exe"
        unix_script = subprocess.check_output([str(MSYS/"usr/bin/cygpath.exe"),"-u",str(script)],text=True).strip()
        run([bash,"--login",unix_script],cwd=WORK,env=env)
    print("Pinned MSYS build packages available.",flush=True)

def main():
    WORK.mkdir(parents=True,exist_ok=True)
    bootstrap()
    prefix = MSYS/"ucrt64"
    env = os.environ.copy()
    env["PATH"] = str(prefix/"bin")+";"+str(MSYS/"usr/bin")+";"+env.get("PATH","")
    env["PKG_CONFIG_PATH"] = str(prefix/"lib/pkgconfig")
    env["PKG_CONFIG_LIBDIR"] = str(prefix/"lib/pkgconfig")
    env["MSYSTEM"]="UCRT64"
    env["CHERE_INVOKING"]="1"
    locks = json.loads((ROOT/"docs/native-source-lock.json").read_text())
    ffmpeg = next(p for p in locks if p["file"]=="mingw-w64-ffmpeg-9.0.1-3.src.tar.zst")
    archive = fetch(ffmpeg,WORK/"corresponding-source/native")
    source = WORK/"ffmpeg-audio-source"
    if not (source/"configure").exists():
        package = WORK/"ffmpeg-source-package"
        package.mkdir(parents=True,exist_ok=True)
        run(["tar.exe","-xf",archive,"-C",package])
        archives=list(package.rglob("ffmpeg-9.0.1.tar.xz"))
        if len(archives)!=1:raise RuntimeError("Expected one pinned FFmpeg source tarball")
        source.mkdir(parents=True,exist_ok=True)
        run(["tar.exe","-xf",archives[0],"--strip-components=1","-C",source])
    build=WORK/"ffmpeg-audio-build"
    build.mkdir(parents=True,exist_ok=True)
    install=WORK/"ffmpeg-audio-install"
    def unix(path):
        return subprocess.check_output([str(MSYS/"usr/bin/cygpath.exe"),"-u",str(path)],text=True).strip()
    script=WORK/"build-ffmpeg-audio.sh"
    configure = shlex.quote(unix(source)+"/configure")
    options = ["--prefix="+unix(install),"--arch=x86_64","--target-os=mingw32","--cc=gcc","--cxx=g++",
        "--enable-shared","--disable-static","--disable-programs","--disable-doc","--disable-debug",
        "--disable-autodetect","--disable-network","--disable-everything","--enable-decoder=aac,alac",
        "--enable-parser=aac","--disable-gpl","--disable-nonfree","--disable-version3"]
    configuration = configure+" "+" ".join(shlex.quote(p) for p in options)
    script.write_text("set -e\ncd "+shlex.quote(unix(build))+
        "\nif [ ! -f ffbuild/config.mak ]; then "+configuration+"; fi\nmake -j4\nmake install\n")
    run([MSYS/"usr/bin/bash.exe","--login",unix(script)],env=env,cwd=WORK)
    cmake = prefix/"bin/cmake.exe"
    run([cmake,"-S",ROOT/"vendor/uxplay","-B",WORK/"uxplay-build","-G","Ninja",
        "-DCMAKE_BUILD_TYPE=Release","-DUSE_MDNS=ON","-DNO_X11_DEPS=ON",
        "-DCMAKE_C_COMPILER="+str(prefix/"bin/gcc.exe"),
        "-DCMAKE_CXX_COMPILER="+str(prefix/"bin/g++.exe"),
        "-DCMAKE_MAKE_PROGRAM="+str(prefix/"bin/ninja.exe")],env=env)
    run([cmake,"--build",WORK/"uxplay-build","--parallel","4"],env=env)
    (WORK/"airplay-build-inputs.json").write_text(json.dumps({
        "msys_lock_sha256":hash_file(ROOT/"docs/msys-build-lock.json"),
        "ffmpeg_configuration":options,"ffmpeg_source_sha256":ffmpeg["sha256"]},indent=2)+"\n")
    print("UxPlay and minimal audio libraries built from source.",flush=True)

if __name__=="__main__":
    main()
