"""Extract original notices from exact MSYS2 source packages without executing build recipes."""
import io, json, subprocess, tarfile
from pathlib import Path, PurePosixPath
ROOT = Path(__file__).resolve().parents[1]
inventory = json.loads((ROOT/"work/notices/inventory.json").read_text())
for package in inventory["native_packages"]:
    if package["notice_files"]:
        continue
    name = package["base"]+"-"+package["version"].split(":")[-1]+".src.tar.zst"
    archive = ROOT/"work/corresponding-source/native"/name
    entries = subprocess.check_output(["tar.exe","-tf",str(archive)],text=True).splitlines()
    count = 0
    for entry in entries:
        if not entry.endswith((".tar.xz",".tar.gz",".tar.bz2")):
            continue
        data = subprocess.check_output(["tar.exe","-xOf",str(archive),entry])
        with tarfile.open(fileobj=io.BytesIO(data)) as source:
            for member in source.getmembers():
                path = PurePosixPath(member.name)
                if not member.isfile() or len(path.parts)>4 or ".." in path.parts:
                    continue
                if not path.name.upper().startswith(("LICENSE","COPYING","NOTICE","AUTHORS")):
                    continue
                target = ROOT/"vendor/licenses"/package["name"]/Path(*path.parts[1:])
                target.parent.mkdir(parents=True,exist_ok=True)
                with source.extractfile(member) as f:
                    target.write_bytes(f.read())
                count += 1
    if not count:
        raise RuntimeError("No source notices: "+name)
    print(package["name"] + ": " + str(count) + " notices")
