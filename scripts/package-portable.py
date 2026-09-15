"""Package an exact hardware-tested stage without rebuilding or changing its binaries.

Python and dumpbin are packaging tools only. The resulting app has no Python runtime.
"""
import argparse
import hashlib
import html
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
GUIDES = ("PORTABLE.vi", "PORTABLE.en", "WDA_SETUP.vi", "WDA_SETUP.en")


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inline(text):
    text = html.escape(text)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", lambda m: '<a href="' +
                  (m[2][:-3]+'.html' if m[2].endswith('.md') else m[2]) +
                  '">' + m[1] + '</a>', text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    return re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)


def guide_page(source):
    # These controlled guides use headings, paragraphs, flat lists and code fences.
    # Escape all text; no Markdown HTML execution, remote assets or scripts.
    blocks, paragraph, code, in_code, listing = [], [], [], False, None

    def flush():
        if paragraph:
            blocks.append('<p>'+inline(' '.join(paragraph))+'</p>')
            paragraph.clear()

    def close_list():
        nonlocal listing
        if listing:
            blocks.append('</'+listing+'>')
            listing = None

    for line in source.splitlines()+['']:
        if line.startswith('```'):
            flush(); close_list()
            if in_code:
                blocks.append('<pre><code>'+html.escape('\n'.join(code))+'</code></pre>')
                code.clear()
            in_code = not in_code
        elif in_code:
            code.append(line)
        elif not line.strip():
            flush(); close_list()
        elif line.startswith('#'):
            flush(); close_list()
            level = len(line)-len(line.lstrip('#'))
            blocks.append(f'<h{level}>'+inline(line[level:].strip())+f'</h{level}>')
        elif re.match(r'^(\d+\. |\- )', line):
            flush()
            kind = 'ul' if line.startswith('- ') else 'ol'
            if listing != kind:
                close_list(); blocks.append('<'+kind+'>'); listing=kind
            blocks.append('<li>'+inline(re.sub(r'^(\d+\. |\- )', '', line))+'</li>')
        elif listing and line.startswith('  '):
            blocks[-1] = blocks[-1][:-5]+' '+inline(line.strip())+'</li>'
        else:
            paragraph.append(line.strip())
    if in_code:
        raise ValueError('Unclosed code fence')
    return '\n'.join(blocks)


def page(title, body, language='en'):
    return f'''<!doctype html><html lang="{language}"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>{html.escape(title)}</title>
<style>:root{{color-scheme:light dark}}body{{font:16px/1.65 "Segoe UI",sans-serif;max-width:850px;margin:auto;padding:28px}}
h1,h2{{line-height:1.3}}h2{{margin-top:2em}}a{{color:light-dark(#005fb8,#8bc6ff)}}
pre{{overflow:auto;padding:16px;background:light-dark(#f2f3f5,#25272a);border-radius:8px}}
code{{font-family:Consolas,monospace}}li{{margin:.5em 0}}nav{{padding:16px;border:1px solid #888;border-radius:8px}}</style>
<body>{body}</body></html>'''


def audit_imports(stage, dumpbin):
    rows = []
    system = Path(os.environ['SystemRoot'])/'System32'
    for binary in sorted(stage.rglob('*')):
        if binary.suffix.lower() not in ('.exe','.dll'):
            continue
        raw = subprocess.check_output([str(dumpbin),'/DEPENDENTS',str(binary)], text=True)
        imports = sorted(set(re.findall(r'^\s+([\w.-]+\.dll)\s*$', raw, re.M|re.I)))
        if not imports:
            raise RuntimeError('Cannot inspect imports: '+binary.name)
        private = [binary.parent]
        if 'AirPlay' in binary.relative_to(stage).parts:
            private += [stage/'AirPlay',stage/'AirPlay/lib/gstreamer-1.0']
        elif binary.parent == stage:
            private += [stage]
        resolved = []
        for name in imports:
            bundled = next((p/name for p in private if (p/name).is_file()),None)
            if bundled:
                location=bundled.relative_to(stage).as_posix()
            elif name.lower().startswith(('api-ms-','ext-ms-')):
                location='Windows API set (OS compatibility still requires testing)'
            elif (system/name).is_file():
                # A developer-installed DLL in System32 is not assumed to be Windows.
                if name.lower().startswith(('vcruntime','msvcp','libusb')):
                    raise RuntimeError('Runtime must be app-local: '+name)
                location='Windows System32 on validation host'
            else:
                raise RuntimeError(f'Unresolved dependency: {binary.name} -> {name}')
            resolved.append({'dll':name,'resolution':location})
        rows.append({'path':binary.relative_to(stage).as_posix(),'sha256':digest(binary),'imports':resolved})
    return rows


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--validated-stage',type=Path,required=True)
    ap.add_argument('--expected-exe-sha256',required=True)
    ap.add_argument('--output-dir',type=Path,required=True)
    ap.add_argument('--dumpbin',type=Path,required=True)
    args=ap.parse_args()
    source=args.validated_stage.resolve()
    output=args.output_dir.resolve()
    if not output.is_relative_to(ROOT/'dist'):
        raise RuntimeError('Output must be a fresh directory inside dist')
    if output.exists():
        raise RuntimeError('Refusing to overwrite an existing release directory')
    if digest(source/'iMirror.exe') != args.expected_exe_sha256.lower():
        raise RuntimeError('EXE differs from the physically validated candidate')
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['package']['version']
    binary_manifest=json.loads((source/'BUILD_MANIFEST.json').read_text())
    for row in binary_manifest['source_files']:
        if row['file'].startswith('crates/') and digest(ROOT/row['file']) != row['sha256'].lower():
            raise RuntimeError('Application source differs from validated build: '+row['file'])
    stage=output/'iMirror'
    stage.mkdir(parents=True)
    # Explicit runtime selection, never the developer's settings, logs or whole dist.
    for name in ('iMirror.exe','iPhoneMirror.UsbConfigurationSwitch.exe','VC_RUNTIME.json','MICROSOFT_RUNTIME_NOTICE.txt','AIRPLAY_BUILD_INPUTS.json'):
        shutil.copy2(source/name,stage/name)
    for path in source.glob('*.dll'):
        shutil.copy2(path,stage/path.name)
    for name in ('AirPlay','WDA','licenses'):
        folder=source/name
        for file in [folder,*folder.rglob('*')]:
            if file.is_symlink() or file.is_junction():
                raise RuntimeError('Runtime filesystem links require review')
            if file.is_file() and (file.suffix.lower() in ('.p12','.pfx','.mobileprovision','.ipa','.dmg','.log') or file.name.lower() in ('setup.json','settings.json','control-status.json')):
                raise RuntimeError('Private/unsupported runtime file: '+file.name)
        shutil.copytree(folder,stage/name)
    shutil.copy2(ROOT/'LICENSE',stage/'LICENSE')
    inventory=json.loads((stage/'licenses/inventory.json').read_text(encoding='utf-8-sig'))
    if inventory.get('missing_notice_files'):
        raise RuntimeError('Missing runtime license notices')
    airplay_build=json.loads((stage/'AIRPLAY_BUILD_INPUTS.json').read_text(encoding='utf-8-sig'))
    if digest(stage/'AirPlay/UxPlay.exe') != airplay_build['helper_sha256'].lower():
        raise RuntimeError('UxPlay does not match recorded build')
    for row in airplay_build['modified_source']:
        path=(ROOT/row['path']).resolve()
        if not path.is_relative_to(ROOT/'vendor/uxplay') or digest(path)!=row['sha256'].lower():
            raise RuntimeError('UxPlay source differs from recorded build')
    for row in inventory['native_files']:
        path=(stage/'AirPlay'/row['path']).resolve()
        if not path.is_relative_to(stage/'AirPlay'):
            raise RuntimeError('Invalid runtime inventory path')
        actual=digest(path)
        if row['path'].lower()=='uxplay.exe':
            row.update(sha256=actual,bytes=path.stat().st_size,build_evidence='AIRPLAY_BUILD_INPUTS.json')
        elif row['sha256'].lower()!=actual:
            raise RuntimeError('Native package provenance mismatch: '+row['path'])
    (stage/'licenses/inventory.json').write_text(json.dumps(inventory,indent=2)+'\n',encoding='utf-8')
    notices=(stage/'licenses/THIRD_PARTY_LICENSES.md').read_text(encoding='utf-8-sig')
    notices+='\n## Additional staged components\n\n'
    notices+='WDA: go-ios (MIT), iMirror forwarder (GPL-3.0-only); all dependency notices are in WDA/licenses and exact provenance in WDA/runtime-manifest.json. The official go-ios CLI has modified revision metadata; exact source reconstruction is not claimed.\n\n'
    notices+='Microsoft Fluent System Icons Regular: MIT; see licenses/fluent-system-icons/LICENSE and NOTICE. Microsoft app-local VC runtime: see MICROSOFT_RUNTIME_NOTICE.txt and VC_RUNTIME.json.\n\n'
    notices+='Corresponding source is a separate release asset containing iMirror/native sources, locked Rust dependencies and forwarder Go dependencies. Sideloadly, Apple images/drivers, signed runners and private setup are not bundled.\n'
    (stage/'THIRD_PARTY_LICENSES.md').write_text(notices,encoding='utf-8')
    (stage/'docs').mkdir()
    for name in GUIDES:
        path=ROOT/'docs'/f'{name}.md'
        shutil.copy2(path,stage/'docs'/path.name)
        (stage/'docs'/f'{name}.html').write_text(page('iMirror guide',guide_page(path.read_text(encoding='utf-8')),'vi' if name.endswith('.vi') else 'en'),encoding='utf-8')
    (stage/'START-HERE.html').write_text(page('iMirror — Start here', '''
<h1>iMirror</h1><p>Windows x64 · Portable engineering preview · Unsigned</p>
<nav><p><a href="docs/PORTABLE.vi.html">Hướng dẫn tiếng Việt: tải, USB, Bluetooth, Wireless</a></p>
<p><a href="docs/PORTABLE.en.html">English: download, USB, Bluetooth, Wireless</a></p>
<p><a href="docs/WDA_SETUP.vi.html">WDA: Sideloadly, Developer Mode, đăng ký thiết bị</a></p>
<p><a href="docs/WDA_SETUP.en.html">WDA: signing, Developer Mode and registration</a></p></nav>
<p>Giải nén tất cả rồi mở iMirror.exe. Keep every runtime file beside the app.</p>
<p>Normal Bluetooth control does not require Apple ID, Developer Mode or WDA.</p>
<p>Sideloadly and Apple images are not bundled. Read the optional WDA guide first.</p>
<p><a href="https://sideloadly.io/">Official Sideloadly download</a> ·
<a href="https://support.apple.com/en-us/118290">Apple Devices</a> ·
<a href="https://github.com/phh235/iphone-mirror-windows/releases">Project releases</a></p>
<p>Clean-machine validation is pending. See RELEASE-STATUS.txt for this exact build.</p>'''),encoding='utf-8')
    (stage/'tools').mkdir()
    shutil.copy2(ROOT/'scripts/portable/Configure-WDA.cmd',stage/'Configure-WDA.cmd')
    for file in (ROOT/'scripts/portable/configure-wda.ps1',ROOT/'scripts/register-wda-runtime.ps1'):
        shutil.copy2(file,stage/'tools'/file.name)
    status=(ROOT/'docs/releases/0.1.0-preview.1.md').read_text(encoding='utf-8')
    (stage/'RELEASE-STATUS.txt').write_text(status,encoding='utf-8')
    imports=audit_imports(stage,args.dumpbin)
    (stage/'PE-DEPENDENCIES.json').write_text(json.dumps(imports,indent=2)+'\n',encoding='utf-8')
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    meta={'app_version':version,'release_channel':'preview.1','source_commit':commit,
          'exe_sha256':digest(stage/'iMirror.exe'),'unchanged_runtime_binaries':len(imports)-1,
          'signed':False,'clean_windows':'UNTESTED','source_archive_required':True,
          'private_configuration_included':False,'experimental_flags_enabled':False}
    (stage/'RELEASE.json').write_text(json.dumps(meta,indent=2)+'\n',encoding='utf-8')
    files=[{'path':p.relative_to(stage).as_posix(),'sha256':digest(p),'bytes':p.stat().st_size}
           for p in sorted(stage.rglob('*')) if p.is_file()]
    (stage/'files.sha256.json').write_text(json.dumps(files,indent=2)+'\n',encoding='utf-8')
    archive=output/f'iMirror-v{version}-windows-x64.zip'
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as z:
        for p in sorted(stage.rglob('*')):
            if p.is_file():z.write(p,'iMirror/'+p.relative_to(stage).as_posix())
    with zipfile.ZipFile(archive) as z:
        if z.testzip():raise RuntimeError('ZIP CRC failure')
        if hashlib.sha256(z.read('iMirror/iMirror.exe')).hexdigest()!=args.expected_exe_sha256.lower():
            raise RuntimeError('Archived EXE changed')
    summary={'zip':str(archive),'zip_bytes':archive.stat().st_size,'sha256':digest(archive),
             'installed_bytes':sum(p.stat().st_size for p in stage.rglob('*') if p.is_file()),
             'files':len(files)+1,'binaries':len(imports),'clean_windows':'UNTESTED'}
    (output/'package-result.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(summary,indent=2))


if __name__=='__main__':
    main()
