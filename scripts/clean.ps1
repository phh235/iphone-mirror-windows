#Requires -Version 7.0
param([string]$KeepRelease, [switch]$Preview, [switch]$KeepResearch)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path.TrimEnd('\')
if (-not (Test-Path -LiteralPath (Join-Path $root 'Cargo.toml')) -or
    -not (Test-Path -LiteralPath (Join-Path $root 'crates'))) { throw 'Not an iMirror workspace.' }
if (-not $KeepRelease) {
    $latest = Get-ChildItem -LiteralPath (Join-Path $root 'dist') -Directory |
        Where-Object { (Test-Path -LiteralPath (Join-Path $_.FullName 'iMirror.exe')) -and
            (Test-Path -LiteralPath (Join-Path $_.FullName 'BUILD_MANIFEST.json')) } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $latest) { throw 'Supply -KeepRelease with a staged release directory.' }
    $KeepRelease = $latest.FullName
}
$release = if ([IO.Path]::IsPathRooted($KeepRelease)) {
    [IO.Path]::GetFullPath($KeepRelease)
} else { [IO.Path]::GetFullPath((Join-Path $root $KeepRelease)) }
if (-not $release.StartsWith($root+'\dist\',[StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath (Join-Path $release 'iMirror.exe'))) { throw 'KeepRelease must be a staged app under dist.' }
$release = (Resolve-Path -LiteralPath $release).Path.TrimEnd('\')
$releaseHash = (Get-FileHash -LiteralPath (Join-Path $release 'iMirror.exe')).Hash
$buildManifest = Join-Path $release 'BUILD_MANIFEST.json'
if (Test-Path -LiteralPath $buildManifest) {
    $build = Get-Content -LiteralPath $buildManifest -Raw | ConvertFrom-Json
    if ($build.exe_sha256 -ne $releaseHash) { throw 'Retained EXE does not match its build manifest.' }
}
# Never erase a file tracked as source, even if a future change places it in a
# directory that was previously just generated output.
$tracked = @(& git -c ('safe.directory='+$root.Replace('\','/')) -C $root ls-files -- target work dist)
if ($LASTEXITCODE -ne 0 -or $tracked.Count -ne 0) { throw 'Generated roots contain tracked files or Git inspection failed.' }
$relative = [Collections.Generic.List[string]]::new()
foreach ($path in @('target','work/dependencies','work/corresponding-source','work/ffmpeg-audio-build',
    'work/ffmpeg-audio-install','work/ffmpeg-audio-source','work/ffmpeg-source-package','work/uxplay-build',
    'work/runtime','work/msys-source-index.html','work/native-ui-checkpoint-source')) { $relative.Add($path) }
$work = Join-Path $root 'work'
if (Test-Path -LiteralPath $work) {
    foreach ($directory in Get-ChildItem -LiteralPath $work -Directory -Force) {
        if ($directory.Name -match '^(package-|video-only-smoke-)') { $relative.Add('work/'+$directory.Name) }
    }
}
foreach ($item in Get-ChildItem -LiteralPath (Join-Path $root 'dist') -Force) {
    if ($item.FullName -eq $release -or $release.StartsWith($item.FullName+'\',[StringComparison]::OrdinalIgnoreCase)) { continue }
    $relative.Add('dist/'+$item.Name)
}
# Research may contain local-only commits even when its worktree is clean.
# Preserve it unconditionally; retain -KeepResearch for command-line compatibility.
$cacheRoots = @($relative | ForEach-Object { [IO.Path]::GetFullPath((Join-Path $root $_)) })
$evidence = @()
$bulky = @()
if (Test-Path -LiteralPath $work) {
    $evidence = @(Get-ChildItem -LiteralPath $work -File -Recurse -Force | Where-Object {
        $file = $_
        $generated = $false
        foreach ($cache in $cacheRoots) {
            if ($file.FullName -eq $cache -or $file.FullName.StartsWith($cache+'\',[StringComparison]::OrdinalIgnoreCase)) { $generated=$true; break }
        }
        -not $generated -and $file.Extension -ne '.zip'
    })
    $bulky = @($evidence | Where-Object { $_.Extension -in @('.bmp','.exe','.dll','.pdb','.obj','.lib') })
    foreach ($file in $bulky) {
        $relative.Add($file.FullName.Substring($root.Length+1))
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
    if ($item.PSIsContainer -and @(Get-ChildItem -LiteralPath $resolved -Recurse -Force -Attributes ReparsePoint).Count -gt 0) {
        throw ('Refusing a cleanup tree containing a reparse point: '+$resolved)
    }
    if ($release -eq $resolved -or $release.StartsWith($resolved+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup overlaps retained release.' }
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
New-Item -ItemType Directory -Force $work | Out-Null
$archivePath = if ($bulky.Count -gt 0) { Join-Path $work ('evidence-before-cleanup-'+(Get-Date -Format 'yyyyMMdd-HHmmss')+'.zip') } else { $null }
$plan = [ordered]@{KeepRelease=$release;ExeSha256=$releaseHash;EvidenceFiles=$evidence.Count;
    EvidenceArchive=$archivePath;Targets=@($targets)}
$plan | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $work 'cleanup-plan.json') -Encoding utf8
Write-Output ('Keep release: '+$release)
Write-Output ('Generated directories: '+@($targets | Where-Object Directory).Count+'; generated files: '+@($targets | Where-Object { -not $_.Directory }).Count)
if ($Preview) { Write-Output 'PREVIEW ONLY: no files deleted or archived.'; return }

# Keep hardware/benchmark evidence before deleting uncompressed captures and old
# test binaries. Small JSON/log/script evidence stays directly accessible as well.
if ($bulky.Count -gt 0) {
    Add-Type -AssemblyName System.IO.Compression
    $records = [Collections.Generic.List[object]]::new()
    $archive = [IO.Compression.ZipFile]::Open($archivePath,[IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in $evidence) {
            $file.Refresh()
            if (($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Evidence file is a reparse point.' }
            $name = $file.FullName.Substring($work.Length+1).Replace('\','/')
            $hash = (Get-FileHash -LiteralPath $file.FullName).Hash
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive,$file.FullName,$name,[IO.Compression.CompressionLevel]::Optimal) | Out-Null
            $records.Add([pscustomobject]@{Path=$name;Bytes=$file.Length;Sha256=$hash})
        }
    } finally { $archive.Dispose() }
    $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
    try {
        foreach ($record in $records) {
            $entry = $archive.GetEntry($record.Path)
            if ($null -eq $entry -or $entry.Length -ne $record.Bytes) { throw 'Evidence archive length mismatch; nothing deleted.' }
            $stream = $entry.Open()
            try { $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)) }
            finally { $stream.Dispose() }
            if ($hash -ne $record.Sha256) { throw 'Evidence archive hash mismatch; nothing deleted.' }
        }
    } finally { $archive.Dispose() }
    $records | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath ($archivePath+'.manifest.json') -Encoding utf8
    Write-Output ('Evidence archived and SHA-256 verified: '+$records.Count+' files')
}
$removed = [Collections.Generic.List[string]]::new()
$processes = @(Get-CimInstance Win32_Process | Where-Object ExecutablePath)
foreach ($target in $targets) {
    if (@($processes | Where-Object { $_.ExecutablePath -eq $target.Path -or $_.ExecutablePath.StartsWith($target.Path+'\',[StringComparison]::OrdinalIgnoreCase) }).Count) {
        throw ('A process started in '+$target.Relative+' during archival; cleanup cancelled.')
    }
}
foreach ($target in $targets) {
    if ($target.Directory) { Remove-Item -LiteralPath $target.Path -Recurse -Force }
    else { Remove-Item -LiteralPath $target.Path -Force }
    $removed.Add($target.Relative)
    if ($target.Directory) { Write-Output ('Removed generated directory: '+$target.Relative) }
}
New-Item -ItemType Directory -Force $work | Out-Null
[ordered]@{Removed=@($removed);KeepRelease=$release;ExeSha256=$releaseHash;EvidenceArchive=$archivePath;
    Preserved='Source, .git and rollback refs, vendor, research, licenses, small benchmark/diagnostic evidence, user settings'} |
    ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $work 'cleanup-report.json') -Encoding utf8
if ((Get-FileHash -LiteralPath (Join-Path $release 'iMirror.exe')).Hash -ne $releaseHash) { throw 'Retained EXE hash changed.' }
Write-Output ('Removed '+$removed.Count+' generated paths. Latest staged app and source were preserved.')
