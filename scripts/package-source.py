"""Assemble corresponding source, build recipes and exact dependency archives."""
import argparse,concurrent.futures,json,os,shutil,subprocess,tempfile,tomllib,zipfile
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
    ap=argparse.ArgumentParser()
    ap.add_argument('--output',type=Path)
    ap.add_argument('--runtime',type=Path,help='Exact staged application, for WDA provenance')
    ap.add_argument('--go',type=Path,help='Go compiler used to build the staged forwarder')
    args=ap.parse_args()
    if args.go:args.go=args.go.resolve()
    if args.runtime:args.runtime=args.runtime.resolve()
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['package']['version']
    native=WORK/"corresponding-source/native"
    lock=json.loads((ROOT/"docs/native-source-lock.json").read_text())
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(lambda row:fetch(row,native),lock))
    cargo=WORK/"corresponding-source/cargo"
    with (WORK/"cargo-vendor-config.txt").open("w") as config:
        subprocess.run(["cargo","vendor","--locked","--versioned-dirs",str(cargo)],cwd=ROOT,stdout=config,check=True)
    output=args.output or ROOT/f"dist/iMirror-{version}-source.zip"
    if output.exists():raise RuntimeError('Refusing to overwrite an existing source archive')
    output.parent.mkdir(parents=True,exist_ok=True)
    # Git inventory excludes private work, diagnostics, signing and build caches.
    tracked=subprocess.check_output(['git','ls-files','-z'],cwd=ROOT).decode().split('\0')
    go_modules=[]
    go_env=os.environ.copy()
    if args.runtime:
        if not args.go:raise RuntimeError('--runtime requires --go for corresponding Go sources')
        go_env.update(GOTOOLCHAIN='local',GOWORK='off',GOMODCACHE=str(WORK/'corresponding-source/go-cache'))
        # go mod download may add transitive checksums; never mutate the checkout.
        with tempfile.TemporaryDirectory(prefix='source-go-',dir=WORK) as temp:
            if not Path(temp).resolve().is_relative_to(WORK.resolve()):raise RuntimeError('Temporary source path outside work')
            for name in ('go.mod','go.sum'):shutil.copy2(ROOT/'vendor/wda-forwarder'/name,Path(temp)/name)
            subprocess.run([str(args.go),'mod','download','all'],cwd=temp,env=go_env,check=True)
            raw=subprocess.check_output([str(args.go),'list','-m','-json','all'],cwd=temp,env=go_env,text=True)
        decoder=json.JSONDecoder()
        while raw.strip():
            row,end=decoder.raw_decode(raw.lstrip());raw=raw.lstrip()[end:]
            if not row.get('Main'):go_modules.append(row)
        if any(not row.get('Dir') for row in go_modules):raise RuntimeError('Missing Go source directory')
    with zipfile.ZipFile(output,"w",compression=zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as z:
        source_inventory=[]
        for name in tracked:
            if not name or name=='.cargo/config.toml':continue
            file=ROOT/name
            if file.is_symlink():raise RuntimeError('Review symlink before source packaging: '+name)
            if file.is_file():
                z.write(file,'iMirror/'+name)
                source_inventory.append({'path':name,'sha256':hash_file(file)})
        z.writestr('iMirror/SOURCE-MANIFEST.json',json.dumps({
            'commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
            'files':source_inventory,'native_archives':lock,
            'go_modules':[{'path':r['Path'],'version':r['Version'],'sum':r.get('Sum')} for r in go_modules]
        },indent=2)+'\n')
        config=(ROOT/".cargo/config.toml").read_text()
        config+='\n[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "dependencies/cargo"\n'
        z.writestr("iMirror/.cargo/config.toml",config)
        for file in cargo.rglob("*"):
            if file.is_file():z.write(file,"iMirror/dependencies/cargo/"+file.relative_to(cargo).as_posix())
        for row in lock:
            file=native/row["file"]
            if hash_file(file)!=row["sha256"]:raise RuntimeError("Source archive hash mismatch")
            z.write(file,"iMirror/work/corresponding-source/native/"+file.name,compress_type=zipfile.ZIP_STORED)
        for row in go_modules:
            folder=Path(row['Dir'])
            for file in folder.rglob('*'):
                if file.is_file():z.write(file,'iMirror/dependencies/go/'+row['Path']+'@'+row['Version']+'/'+file.relative_to(folder).as_posix())
        if args.runtime:
            z.write(args.runtime/'WDA/runtime-manifest.json','iMirror/distribution/WDA-runtime-manifest.json')
            # Cache archives let Go reconstruct module-cache checksums offline.
            for file in (WORK/'corresponding-source/go-cache/cache/download').rglob('*'):
                if file.is_file() and file.suffix in ('.zip','.mod','.info','.ziphash'):
                    z.write(file,'iMirror/dependencies/go-download/'+file.relative_to(WORK/'corresponding-source/go-cache/cache/download').as_posix())
            z.writestr('iMirror/distribution/GO-SOURCE.txt',
                'Forwarder: exact go.mod/go.sum dependency sources and module-download cache are included.\n'
                'Set GOPROXY=file:///<absolute path to dependencies/go-download>, GOSUMDB=off, GOTOOLCHAIN=local; use pinned Go from prepare-wda-runtime.ps1.\n'
                'Run go build -mod=readonly -trimpath -ldflags "-s -w" in vendor/wda-forwarder.\n'
                'go-ios CLI is the separate MIT upstream release asset. Its embedded revision is dirty; this archive does not claim exact source reproduction of that CLI. See WDA runtime manifest and notices.\n')
        z.writestr("iMirror/SOURCE_BUILD.txt",
            "This archive contains the application, native patches, every Cargo.lock dependency, and the exact native library source packages.\n"
            "Run powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1 on a Windows x64 build host.\n"
            "Rust MSVC, VS C++ Build Tools, CMake and Python are build-time prerequisites. The pinned MSYS and WiX build tool archives download on first use.\n"
            "Native .src.tar.zst files include the original upstream tarballs and MSYS2 PKGBUILD recipes; do not omit them when redistributing this source archive.\n")
    print(f"Source archive: {output} ({output.stat().st_size/1048576:.1f} MiB)")
if __name__=="__main__":main()
