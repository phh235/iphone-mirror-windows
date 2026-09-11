#Requires -Version 7.0
param([switch]$KeepResearch)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path.TrimEnd('\')
if (-not (Test-Path -LiteralPath (Join-Path $root 'Cargo.toml')) -or
    -not (Test-Path -LiteralPath (Join-Path $root 'crates'))) { throw 'Not an iMirror workspace.' }
if (-not (Test-Path -LiteralPath (Join-Path $root 'dist/portable/iMirror.exe'))) {
    throw 'Preserve a runnable release in dist/portable before cleaning build caches.'
}
$relative = [Collections.Generic.List[string]]::new()
foreach ($path in @('target','work/dependencies','work/corresponding-source','work/ffmpeg-audio-build',
    'work/ffmpeg-audio-install','work/ffmpeg-audio-source','work/ffmpeg-source-package','work/uxplay-build',
    'work/runtime','work/msys-source-index.html')) { $relative.Add($path) }
$work = Join-Path $root 'work'
if (Test-Path -LiteralPath $work) {
    foreach ($directory in Get-ChildItem -LiteralPath $work -Directory -Force) {
        if ($directory.Name -match '^(package-|video-only-smoke-)') { $relative.Add('work/'+$directory.Name) }
    }
}
foreach ($name in @('iMirror-0.1.0-Setup.exe','iMirror-0.1.0-Setup.wixpdb','iMirror-0.1.0-x64.msi',
    'iMirror-0.1.0-x64.wixpdb','iMirror-0.1.0-x64.zip','iMirror-0.1.0-source.zip','latest-stage.txt','SHA256SUMS.txt')) {
    $relative.Add('dist/'+$name)
}
$skipped = [Collections.Generic.List[string]]::new()
if (-not $KeepResearch -and (Test-Path -LiteralPath (Join-Path $root 'research'))) {
    foreach ($directory in Get-ChildItem -LiteralPath (Join-Path $root 'research') -Directory -Force) {
        if (-not (Test-Path -LiteralPath (Join-Path $directory.FullName '.git'))) { continue }
        if (($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Research root is a reparse point.' }
        $dirty = @(& git -c ('safe.directory='+$directory.FullName.Replace('\','/')) -C $directory.FullName status --porcelain)
        if ($LASTEXITCODE -ne 0) { throw ('Could not inspect '+$directory.Name) }
        if ($dirty.Count -gt 0) { $skipped.Add('research/'+$directory.Name); continue }
        $relative.Add('research/'+$directory.Name)
    }
}
# Resolve and verify every recursive target before any removal. Never target source,
# .git, user settings, or anything outside this workspace. PowerShell 7 does not
# recurse through directory symlinks during Remove-Item.
$targets = [Collections.Generic.List[object]]::new()
foreach ($path in $relative) {
    $full = [IO.Path]::GetFullPath((Join-Path $root $path))
    if (-not $full.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup target escaped workspace.' }
    if (-not (Test-Path -LiteralPath $full)) { continue }
    $item = Get-Item -LiteralPath $full -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw ('Refusing reparse target: '+$full) }
    $resolved = (Resolve-Path -LiteralPath $full).Path
    if (-not $resolved.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Resolved target escaped workspace.' }
    $ancestor = Split-Path $resolved -Parent
    while ($ancestor -ne $root) {
        $parent = Get-Item -LiteralPath $ancestor -Force
        if (($parent.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Cleanup path traverses a reparse point.' }
        $ancestor = Split-Path $ancestor -Parent
    }
    $targets.Add([pscustomobject]@{Relative=$path;Path=$resolved;Directory=$item.PSIsContainer})
}
$processes = @(Get-CimInstance Win32_Process | Where-Object ExecutablePath)
foreach ($target in $targets) {
    foreach ($process in $processes) {
        if ($process.ExecutablePath.Equals($target.Path,[StringComparison]::OrdinalIgnoreCase) -or
            $process.ExecutablePath.StartsWith($target.Path+'\',[StringComparison]::OrdinalIgnoreCase)) {
            throw ('Close process '+$process.ProcessId+' before cleaning '+$target.Relative)
        }
    }
}
$removed = [Collections.Generic.List[string]]::new()
foreach ($target in $targets) {
    if ($target.Directory) { Remove-Item -LiteralPath $target.Path -Recurse -Force }
    else { Remove-Item -LiteralPath $target.Path -Force }
    $removed.Add($target.Relative)
}
New-Item -ItemType Directory -Force $work | Out-Null
[ordered]@{Removed=@($removed);PreservedDirtyResearch=@($skipped);Portable='dist/portable/iMirror.exe';
    Preserved='Source, .git, licenses, benchmark evidence, work-root diagnostics, user settings'} |
    ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $work 'cleanup-report.json') -Encoding utf8
Write-Output ('Removed '+$removed.Count+' generated paths. Portable release and source were preserved.')
if ($skipped.Count -gt 0) { Write-Output ('Preserved modified research clones: '+($skipped -join ', ')) }
