param([switch]$Check)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    $raw = & cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE) { throw 'Cargo license metadata failed; fetch the locked dependencies first.' }
    $metadata = ($raw -join "`n") | ConvertFrom-Json
    # Culture/.NET-dependent Sort-Object ordering differs between Windows
    # PowerShell and CI's PowerShell 7 (for example, http-body vs httparse).
    $packages = [Collections.Generic.List[object]]::new()
    foreach ($package in ($metadata.packages | Where-Object source)) { $packages.Add($package) }
    $packages.Sort([Comparison[object]]{
        param($left, $right)
        foreach ($field in @('name','version','id')) {
            $order = [StringComparer]::Ordinal.Compare([string]$left.$field, [string]$right.$field)
            if ($order -ne 0) { return $order }
        }
        return 0
    })
    $lines = [Collections.Generic.List[string]]::new()
    $intro = @'
# Third-party licenses

This is the **source dependency index**, generated from Cargo.lock and the pinned
components retained in this repository. It includes build/proc-macro dependencies;
it is not a claim that every listed crate is loaded at runtime. License expressions
are reproduced as declared by the package manifests.

iMirror is [GPL-3.0-only](LICENSE). Binary packages must also include the exact
runtime inventory, full notices and corresponding source required for their
contents. See [distribution requirements](docs/LICENSING.md). The release script
generates a separate package-specific inventory under `licenses/`.

## Retained native code, assets and build components

| Component | Revision/source | Terms and notice |
| --- | --- | --- |
| iPhoneMirror native core | `2635a0073c67539aec86487844fc41a5caddf9ba` | [GPL-3.0-only](vendor/iphone-mirror/LICENSE); [local patches](vendor/iphone-mirror/PATCHES.md) |
| UxPlay | `f2c4a66e704859e791e139db4f3fdba79cd838f9` | [GPL-3.0-or-later](vendor/uxplay/LICENSE); retained component notices also apply |
| UxPlay llhttp | Pinned UxPlay tree | [MIT](vendor/uxplay/lib/llhttp/LICENSE-MIT) |
| UxPlay PlayFair | Pinned UxPlay tree | [GPL notice](vendor/uxplay/lib/playfair/LICENSE.md) |
| UxPlay mdnsd | Pinned UxPlay tree | [LGPL-2.1-or-later header](vendor/uxplay/lib/mdnsd/mdnsd.c) |
| libusb | Pinned native-core third_party files | [LGPL-2.1-or-later](vendor/iphone-mirror/third_party/libusb/COPYING) |
| libusb-win32 | Pinned native-core third_party files | [LGPL-3.0](vendor/iphone-mirror/third_party/libusb-win32/COPYING_LGPL.txt) |
| QuickTime protocol fixtures | See fixture provenance | [MIT](vendor/iphone-mirror/src/Core/tests/fixtures/quicktime_video_hack/LICENSE); [provenance](vendor/iphone-mirror/src/Core/tests/fixtures/README.md) |
| windows-ble-hid report-design reference | `9a4f45129779ec3bef368ea4ffcdcde795a57b65` | [MIT](vendor/licenses/windows-ble-hid-MIT.txt) |
| Microsoft Fluent System Icons | `9cf8af0f95a555918a60b8147a2f33a6a1248442` | [MIT](assets/fluent/LICENSE), [NOTICE](assets/fluent/NOTICE), [asset hashes](assets/fluent/SHA256SUMS.txt) |
| WiX build/bootstrapper components | 3.14.1 | [Retained WiX license and exceptions](vendor/licenses/wix3/LICENSE.TXT) |
| Microsoft VC runtime | Official app-local deployment, selected at package build | [Redistribution documentation](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution); runtime DLLs are not checked into this source repository |

GStreamer, minimal FFmpeg audio libraries and other private wireless runtime
dependencies are rebuilt/staged by the packaging scripts. Audio playback in
iMirror is disabled; that does not make retained runtime license obligations
disappear. Exact native package archives are pinned in
[native-source-lock.json](docs/native-source-lock.json) and build packages in
[msys-build-lock.json](docs/msys-build-lock.json). Their staged-file hashes and
notices must be regenerated for an actual release, not inferred from this table.

The optional managed WDA runtime stages the MIT-licensed go-ios Windows CLI and
the GPL-3.0-only [iMirror forwarder](vendor/wda-forwarder/README.md). The forwarder
uses go-ios v1.3.2 through its locked Go module graph. Its staging script copies
the Go SDK and dependency licenses into `WDA/licenses` and records exact binary
hashes and embedded module versions in `WDA/runtime-manifest.json`. The official
CLI asset reports modified upstream source; it is hash-pinned, not claimed to
be rebuilt from the source tag. See [provenance and release limits](docs/WDA_MANAGED_RUNTIME.md).

The signed WebDriverAgent phone runner remains a separately installed prerequisite,
not a bundled app. Apple proprietary software, credentials, pairing records and
non-OSI iUsbBridge components are excluded.

## Rust packages from locked Windows x64 metadata

| Package | Version | Declared license | Upstream |
| --- | --- | --- | --- |
'@
    foreach ($line in ($intro -split "`r?`n")) { $lines.Add($line) }
    foreach ($package in $packages) {
        if (-not $package.license) { throw ('Review missing license metadata: '+$package.name) }
        $url = if ($package.repository) { $package.repository } else { 'https://crates.io/crates/'+$package.name+'/'+$package.version }
        $lines.Add('| '+$package.name+' | '+$package.version+' | '+$package.license+' | [source]('+$url+') |')
    }
    $lines.Add('')
    $lines.Add('Regenerate with `pwsh -File scripts/update-license-index.ps1`; verify with `-Check`.')
    $lines.Add('Full third-party notices for binary distributions are collected by `scripts/runtime-inventory.py`.')
    $lines.Add('Preserve the special upstream notice handling recorded under `vendor/licenses/`.')
    $text = ($lines -join "`n")+"`n"
    $path = Join-Path $root 'THIRD_PARTY_LICENSES.md'
    if ($Check) {
        if (-not (Test-Path -LiteralPath $path) -or ([IO.File]::ReadAllText($path).Replace("`r`n","`n")) -ne $text) {
            throw 'THIRD_PARTY_LICENSES.md is stale; run update-license-index.ps1.'
        }
        Write-Output ('License index matches '+$packages.Count+' external Cargo packages.')
    } else {
        [IO.File]::WriteAllText($path,$text,(New-Object Text.UTF8Encoding($false)))
        Write-Output ('Wrote source license index for '+$packages.Count+' external Cargo packages.')
    }
} finally { Pop-Location }
