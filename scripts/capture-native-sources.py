import concurrent.futures
import hashlib
import json
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[1]
target = ROOT / "work/corresponding-source/native"
target.mkdir(parents=True, exist_ok=True)
inventory = json.loads((ROOT/"work/notices/inventory.json").read_text())
index = (ROOT/"work/msys-source-index.html").read_text()
names = {p["base"] + "-" + p["version"].split(":")[-1] + ".src.tar.zst" for p in inventory["native_packages"]}
names.add("mingw-w64-ffmpeg-9.0.1-3.src.tar.zst")
for name in names:
    if ('href="' + name + '"') not in index:
        raise RuntimeError("Source archive unavailable: " + name)
def fetch(name):
    url = "https://repo.msys2.org/mingw/sources/" + name
    path = target/name
    if not path.exists():
        subprocess.run(["curl.exe","--fail","--location","--silent","--show-error","--retry","2","--output",str(path),url],check=True)
    with path.open("rb") as f:
        sha = hashlib.file_digest(f,"sha256").hexdigest()
    return {"file":name,"url":url,"sha256":sha,"bytes":path.stat().st_size}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    rows = sorted(pool.map(fetch, sorted(names)),key=lambda p:p["file"])
(ROOT/"docs/native-source-lock.json").write_text(json.dumps(rows,indent=2)+"\n")
print(f"Fetched {len(rows)} exact source packages ({sum(r['bytes'] for r in rows)/1048576:.1f} MiB).")
