param([string]$Destination)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (-not $Destination) { $Destination = Join-Path $root 'work/runtime/AirPlay' }
$destinationRoot = [IO.Path]::GetFullPath($Destination)
if (-not $destinationRoot.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase) -or
    [IO.Path]::GetFileName($destinationRoot) -ne 'AirPlay') { throw 'Runtime destination must be a workspace AirPlay directory.' }
$prefix = Join-Path $root 'work/dependencies/msys64/ucrt64'
$mini = Join-Path $root 'work/ffmpeg-audio-install/bin'
$objdump = Join-Path $prefix 'bin/objdump.exe'
$helper = Join-Path $root 'work/uxplay-build/uxplay.exe'
if (-not (Test-Path $helper)) { throw 'Build the pinned UxPlay source first.' }
if (-not (Test-Path (Join-Path $mini 'avcodec-63.dll'))) { throw 'Build the minimal AAC/ALAC libraries first.' }
$marker = Join-Path $destinationRoot '.imirror-generated'
if ((Test-Path $destinationRoot) -and -not (Test-Path $marker) -and
    @(Get-ChildItem -LiteralPath $destinationRoot -Force).Count -gt 0) { throw 'Refusing to modify an unmarked runtime directory.' }
New-Item -ItemType Directory -Force $destinationRoot,(Join-Path $destinationRoot 'lib/gstreamer-1.0') | Out-Null
Set-Content -LiteralPath $marker -Value 'Generated iMirror native runtime' -Encoding utf8
Copy-Item -LiteralPath $helper -Destination (Join-Path $destinationRoot 'UxPlay.exe') -Force
$plugins = @('autodetect','app','coreelements','videoparsersbad','rtp','rtpmanager','udp','audioparsers',
    'audioconvert','audioresample','libav','playback','typefindfunctions','volume',
    'videoconvertscale','jpeg','imagefreeze','pango')
$queue = [Collections.Generic.Queue[string]]::new()
$queue.Enqueue((Join-Path $destinationRoot 'UxPlay.exe'))
foreach ($plugin in $plugins) {
    $file = 'libgst' + $plugin + '.dll'
    $source = Join-Path $prefix ('lib/gstreamer-1.0/' + $file)
    $target = Join-Path $destinationRoot ('lib/gstreamer-1.0/' + $file)
    Copy-Item -LiteralPath $source -Destination $target -Force
    $queue.Enqueue($target)
}
$visited = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$system = @('kernel32.dll','kernelbase.dll','ntdll.dll','advapi32.dll','user32.dll','gdi32.dll',
    'ws2_32.dll','wsock32.dll','rpcrt4.dll','bcrypt.dll','bcryptprimitives.dll','winmm.dll','shell32.dll',
    'shlwapi.dll','shcore.dll','ole32.dll','oleaut32.dll','combase.dll','cfgmgr32.dll','setupapi.dll',
    'iphlpapi.dll','dnsapi.dll','powrprof.dll','secur32.dll','crypt32.dll','cryptbase.dll','ncrypt.dll',
    'normaliz.dll','winhttp.dll','wininet.dll','usp10.dll','version.dll','runtimeobject.dll','profapi.dll',
    'clbcatq.dll','propsys.dll','d3d11.dll','d3d12.dll','dxgi.dll','d3dcompiler_47.dll','dwmapi.dll',
    'avrt.dll','mfplat.dll','mf.dll','mfuuid.dll','msvcrt.dll','ucrtbase.dll','comdlg32.dll','comctl32.dll',
    'imm32.dll','opengl32.dll','glu32.dll','d2d1.dll','dwrite.dll','dxva2.dll','hid.dll','winspool.drv',
    'wtsapi32.dll','netapi32.dll','userenv.dll','authz.dll','msimg32.dll','msvfw32.dll','vfw32.dll',
    'wintrust.dll','ksuser.dll','psapi.dll','dbghelp.dll','wldap32.dll','mswsock.dll','dcomp.dll',
    'uxtheme.dll','mpr.dll','dsound.dll','msdmo.dll')
while ($queue.Count -gt 0) {
    $file = $queue.Dequeue()
    if (-not $visited.Add($file)) { continue }
    $dump = & $objdump -p $file
    if ($LASTEXITCODE -ne 0) { throw "PE inspection failed: $file" }
    foreach ($line in $dump) {
        if ($line -notmatch '^\s*DLL Name:\s*(.+?)\s*$') { continue }
        $name = $Matches[1]
        if ($name -match '^(api-ms-win-|ext-ms-win-)' -or $system -contains $name) { continue }
        if ([IO.Path]::GetFileName($name) -ne $name) { throw "Invalid imported DLL name: $name" }
        $target = Join-Path $destinationRoot $name
        if (-not $visited.Contains($target)) {
            $source = Join-Path $mini $name
            if (-not (Test-Path $source)) { $source = Join-Path $prefix ('bin/' + $name) }
            if (-not (Test-Path $source)) { throw "Unresolved runtime dependency: $name from $file" }
            Copy-Item -LiteralPath $source -Destination $target -Force
            $queue.Enqueue($target)
        }
    }
}
# Only remove obsolete generated binary files, after checking each absolute path.
foreach ($file in Get-ChildItem -LiteralPath $destinationRoot -Recurse -File) {
    if ($file.Extension -notin @('.dll','.exe') -or $visited.Contains($file.FullName)) { continue }
    if (-not $file.FullName.StartsWith($destinationRoot+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe runtime cleanup path' }
    Remove-Item -LiteralPath $file.FullName -Force
}
$manifest = @(Get-ChildItem -LiteralPath $destinationRoot -File -Recurse | Where-Object Name -ne 'runtime-manifest.json' | ForEach-Object {
    [ordered]@{ path = $_.FullName.Substring($destinationRoot.Length+1).Replace('\','/');
        sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(); bytes = $_.Length }
})
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $destinationRoot 'runtime-manifest.json') -Encoding utf8
Write-Output ("Staged " + $manifest.Count + " native files, " + [math]::Round(($manifest.bytes | Measure-Object -Sum).Sum/1MB,2) + " MiB.")
