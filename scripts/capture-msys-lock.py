import hashlib,json,subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
MSYS=ROOT/"work/dependencies/msys64"
def fields(path):
    result={}; key=None
    for line in path.read_text().splitlines():
        if line.startswith("%") and line.endswith("%"):
            key=line.strip("%"); result[key]=[]
        elif key and line:
            result[key].append(line)
    return result
def sha(path):
    with path.open("rb") as f: return hashlib.file_digest(f,"sha256").hexdigest()
base=ROOT/"work/dependencies/msys2-base-x86_64-20260611.tar.xz"
base_entries=set(subprocess.check_output(["tar.exe","-tf",str(base)],text=True).splitlines())
rows=[]; builtin=[]; missing=[]
for folder in sorted((MSYS/"var/lib/pacman/local").iterdir()):
    if not (folder/"desc").exists():continue
    d=fields(folder/"desc"); name=d["NAME"][0]; version=d["VERSION"][0]
    files=list((MSYS/"var/cache/pacman/pkg").glob(name+"-"+version.split(":")[-1]+"-*.pkg.tar.zst"))
    if not files:
        if any("var/lib/pacman/local/"+folder.name+"/desc" in entry for entry in base_entries):
            builtin.append({"name":name,"version":version})
        else:missing.append(name+" "+version)
        continue
    if len(files)!=1: raise RuntimeError("Ambiguous package "+name)
    p=files[0]
    repository="mingw/ucrt64" if name.startswith("mingw-w64-ucrt-") else "msys/x86_64"
    rows.append({"name":name,"version":version,"file":p.name,
        "url":"https://repo.msys2.org/"+repository+"/"+p.name,"sha256":sha(p),"bytes":p.stat().st_size})
lock={"base":{"file":base.name,"url":"https://repo.msys2.org/distrib/x86_64/"+base.name,"sha256":sha(base),"bytes":base.stat().st_size},
      "packages":rows,"included_in_base":builtin,"missing":missing}
(ROOT/"docs/msys-build-lock.json").write_text(json.dumps(lock,indent=2)+"\n")
print(json.dumps({"packages":len(rows),"base_packages":len(builtin),"missing":missing,"MiB":sum(p["bytes"] for p in rows)/1048576},indent=2))
