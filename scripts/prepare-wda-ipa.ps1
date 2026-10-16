#Requires -Version 5.1
param(
    [Parameter(Mandatory=$true)][string]$UpstreamArchive,
    [Parameter(Mandatory=$true)][string]$OutputIpa
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$root = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path.TrimEnd('\')
$sourcePath = (Resolve-Path -LiteralPath $UpstreamArchive).Path
$outputPath = [IO.Path]::GetFullPath($OutputIpa)
if (-not $outputPath.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase) -or
    [IO.Path]::GetExtension($outputPath) -ne '.ipa') { throw 'Output must be a new .ipa inside the workspace.' }
if (Test-Path -LiteralPath $outputPath) { throw 'Refusing to overwrite an existing IPA.' }
if ((Get-FileHash -LiteralPath $sourcePath).Hash -ne '4D16FE8D452B675B2B57D924826AF2AD0E1F1713CE257FBF70A023D4243BDE59') {
    throw 'Expected the pinned Appium WebDriverAgent v16.12.8 real-device archive.'
}
New-Item -ItemType Directory -Force -Path (Split-Path $outputPath -Parent) | Out-Null
$source = [IO.Compression.ZipFile]::OpenRead($sourcePath)
$appRoot = 'WebDriverAgentRunner-Runner.app/'
$kept = [Collections.Generic.List[object]]::new()
$removed = [Collections.Generic.List[string]]::new()
$names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
try {
    foreach ($entry in $source.Entries) {
        if (-not $entry.FullName.StartsWith($appRoot,[StringComparison]::Ordinal) -or
            $entry.FullName.Contains('\') -or $entry.FullName.Contains(':') -or
            @($entry.FullName.Split('/') | Where-Object { $_ -in @('.','..') }).Count -or
            -not $names.Add($entry.FullName)) { throw 'Unexpected or duplicate upstream archive path.' }
        if (@($entry.FullName.Split('/') | Where-Object { $_.EndsWith('.dSYM',[StringComparison]::OrdinalIgnoreCase) }).Count) {
            $removed.Add($entry.FullName)
        } else { $kept.Add($entry) }
    }
    foreach ($path in @(
        'Info.plist', 'WebDriverAgentRunner-Runner',
        'PlugIns/WebDriverAgentRunner.xctest/Info.plist',
        'PlugIns/WebDriverAgentRunner.xctest/WebDriverAgentRunner',
        'PlugIns/WebDriverAgentRunner.xctest/Frameworks/WebDriverAgentLib.framework/Info.plist',
        'PlugIns/WebDriverAgentRunner.xctest/Frameworks/WebDriverAgentLib.framework/WebDriverAgentLib'
    )) {
        if (-not $source.GetEntry($appRoot+$path)) { throw ('Missing required runner file: '+$path) }
    }
    $target = [IO.Compression.ZipFile]::Open($outputPath,[IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($entry in $kept) {
            $copy = $target.CreateEntry('Payload/'+$entry.FullName,[IO.Compression.CompressionLevel]::Optimal)
            $copy.ExternalAttributes = $entry.ExternalAttributes
            $copy.LastWriteTime = $entry.LastWriteTime
            $input = $entry.Open(); $output = $copy.Open()
            try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
        }
    } finally { $target.Dispose() }
    $check = [IO.Compression.ZipFile]::OpenRead($outputPath)
    try {
        if ($check.Entries.Count -ne $kept.Count) { throw 'Unexpected output entry count.' }
        foreach ($entry in $kept) {
            $copy = $check.GetEntry('Payload/'+$entry.FullName)
            if (-not $copy -or $copy.Length -ne $entry.Length) { throw 'Output layout/length mismatch.' }
            $a = $entry.Open(); $b = $copy.Open(); $hash = [Security.Cryptography.SHA256]::Create()
            try {
                if ([BitConverter]::ToString($hash.ComputeHash($a)) -ne [BitConverter]::ToString($hash.ComputeHash($b))) {
                    throw ('Runtime file changed: '+$entry.FullName)
                }
            } finally { $hash.Dispose(); $a.Dispose(); $b.Dispose() }
        }
        if (@($check.Entries | Where-Object { $_.FullName -match '(?i)\.dSYM(/|$)' }).Count) { throw 'Debug symbols remain in IPA.' }
    } finally { $check.Dispose() }
} finally { $source.Dispose() }
$result = [ordered]@{
    input_sha256=(Get-FileHash -LiteralPath $sourcePath).Hash.ToLowerInvariant()
    output_sha256=(Get-FileHash -LiteralPath $outputPath).Hash.ToLowerInvariant()
    bytes=(Get-Item -LiteralPath $outputPath).Length
    removed_debug_entries=@($removed)
    retained_entries=$kept.Count
    retained_entry_hashes_match_upstream=$true
    signed=$false
    sideloadly_sign_install='UNTESTED - must be completed by the device owner'
    physical_wda_launch='UNTESTED'
}
[IO.File]::WriteAllText(($outputPath+'.validation.json'),($result|ConvertTo-Json -Depth 4),[Text.UTF8Encoding]::new($false))
$result|ConvertTo-Json -Depth 4
