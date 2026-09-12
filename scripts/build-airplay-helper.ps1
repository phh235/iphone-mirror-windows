#Requires -Version 7.0
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$work = Join-Path $root 'work/airplay-helper-build'
$cache = Join-Path $work 'packages'
$sdk = Join-Path $work 'sdk'
New-Item -ItemType Directory -Path $cache,$sdk -Force | Out-Null
$lockPath = Join-Path $root 'docs/msys-build-lock.json'
$lock = Get-Content -LiteralPath $lockPath -Raw | ConvertFrom-Json
if ($lock.missing.Count) { throw 'The native build lock is incomplete.' }
$packages = @{}
foreach ($entry in $lock.packages) { $packages[$entry.name] = $entry }
$prefix = 'mingw-w64-ucrt-x86_64-'
$pending = [Collections.Generic.Queue[string]]::new()
# Explicit providers for the cc-libs and libjpeg virtual dependencies in this lock.
# Their .PKGINFO provides records are still read before dependency traversal.
foreach ($name in @('gcc-libs','gcc','pkgconf','openssl','libjpeg-turbo','libplist','gstreamer','gst-plugins-base','cmake','ninja','winpthreads')) { $pending.Enqueue($prefix+$name) }
$visited = [Collections.Generic.HashSet[string]]::new()
$providers = @{}
$selected = [Collections.Generic.List[object]]::new()
while ($pending.Count) {
    $batch = @()
    while ($pending.Count -and $batch.Count -lt 4) {
        $name = $pending.Dequeue()
        if (!$packages.ContainsKey($name) -and $providers.ContainsKey($name)) { $name = $providers[$name] }
        if (!$packages.ContainsKey($name)) { throw ('Dependency is not in the pinned lock: '+$name) }
        if (!$visited.Add($name)) { continue }
        $batch += $packages[$name]
    }
    if (!$batch.Count) { continue }
    $downloads = @($batch | ForEach-Object -Parallel {
        $entry = $_
        if ([IO.Path]::GetFileName($entry.file) -ne $entry.file) { throw 'Expected a package filename without path components.' }
        $target = Join-Path $using:cache $entry.file
        if (!(Test-Path -LiteralPath $target) -or (Get-FileHash -LiteralPath $target).Hash -ne $entry.sha256) {
            $temporary = $target+'.download'
            & curl.exe --fail --location --silent --show-error --retry 2 --output $temporary $entry.url
            if ($LASTEXITCODE -ne 0) { throw ('Download failed: '+$entry.file) }
            if ((Get-FileHash -LiteralPath $temporary).Hash -ne $entry.sha256) { throw ('Checksum mismatch: '+$entry.file) }
            Move-Item -LiteralPath $temporary -Destination $target -Force
        }
        [pscustomobject]@{entry=$entry;archive=$target}
    } -ThrottleLimit 4)
    if ($downloads.Count -ne $batch.Count) { throw 'A pinned package download did not complete.' }
    # Extraction stays sequential: packages share the SDK tree. Package install
    # scripts are not run; only the native ucrt64 headers/libraries/tools are used.
    foreach ($download in $downloads) {
        $info = @(& tar.exe -xOf $download.archive .PKGINFO)
        if ($LASTEXITCODE -ne 0) { throw 'Could not inspect pinned package metadata.' }
        foreach ($line in $info) {
            if ($line -match '^provides = (.+)$') { $providers[($Matches[1] -split '[<>=]',2)[0]] = $download.entry.name }
            if ($line -match '^depend = (.+)$') {
                $dependency = ($Matches[1] -split '[<>=]',2)[0]
                # MSYS shell dependencies only support package install hooks,
                # which this native SDK extraction deliberately does not run.
                if ($dependency.StartsWith($prefix,[StringComparison]::Ordinal)) { $pending.Enqueue($dependency) }
            }
        }
        $markerDirectory = Join-Path $sdk '.packages'
        New-Item -ItemType Directory -Path $markerDirectory -Force | Out-Null
        $marker = Join-Path $markerDirectory ($download.entry.file+'.sha256')
        if (!(Test-Path -LiteralPath $marker) -or (Get-Content -LiteralPath $marker -Raw).Trim() -ne $download.entry.sha256) {
            $paths = @(& tar.exe -tf $download.archive)
            if ($LASTEXITCODE -ne 0) { throw 'Could not inspect pinned archive paths.' }
            foreach ($path in $paths) {
                if ($path -in @('.PKGINFO','.BUILDINFO','.MTREE','.INSTALL')) { continue }
                if ($path.Contains('\') -or $path.Split('/').Contains('..') -or
                    !($path.StartsWith('ucrt64/',[StringComparison]::Ordinal) -or $path.StartsWith('usr/',[StringComparison]::Ordinal))) { throw ('Unexpected native SDK archive path in '+$download.entry.name+': '+$path) }
            }
            & tar.exe -xf $download.archive -C $sdk ucrt64
            if ($LASTEXITCODE -ne 0) { throw ('Native SDK extraction failed: '+$download.entry.file) }
            [IO.File]::WriteAllText($marker,$download.entry.sha256,[Text.UTF8Encoding]::new($false))
        }
        $selected.Add($download.entry)
        Write-Output ('Pinned SDK package: '+$download.entry.name+' '+$download.entry.version)
    }
}
$toolchain = Join-Path $sdk 'ucrt64'
$env:PATH = (Join-Path $toolchain 'bin')+';'+(Join-Path $env:SystemRoot 'System32')
$env:PKG_CONFIG_PATH = Join-Path $toolchain 'lib/pkgconfig'
$env:PKG_CONFIG_LIBDIR = $env:PKG_CONFIG_PATH
$cmake = Join-Path $toolchain 'bin/cmake.exe'
$build = Join-Path $work 'build'
& $cmake -S (Join-Path $root 'vendor/uxplay') -B $build -G Ninja -DCMAKE_BUILD_TYPE=Release -DUSE_MDNS=ON -DNO_X11_DEPS=ON ('-DCMAKE_C_COMPILER='+(Join-Path $toolchain 'bin/gcc.exe')) ('-DCMAKE_CXX_COMPILER='+(Join-Path $toolchain 'bin/g++.exe')) ('-DCMAKE_MAKE_PROGRAM='+(Join-Path $toolchain 'bin/ninja.exe')) ('-DPKG_CONFIG_EXECUTABLE='+(Join-Path $toolchain 'bin/pkg-config.exe'))
if ($LASTEXITCODE -ne 0) { throw 'UxPlay configure failed.' }
& $cmake --build $build --parallel 4
if ($LASTEXITCODE -ne 0) { throw 'UxPlay build failed.' }
$manifest = [ordered]@{lock_sha256=(Get-FileHash -LiteralPath $lockPath).Hash;packages=$selected.ToArray();helper_sha256=(Get-FileHash -LiteralPath (Join-Path $build 'uxplay.exe')).Hash;scope='UxPlay helper only; no Rust, USB, decoder or runtime DLL rebuild.'}
[IO.File]::WriteAllText((Join-Path $work 'build-inputs.json'),($manifest|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
Write-Output ('Built helper: '+(Join-Path $build 'uxplay.exe'))
