param([Parameter(Mandatory=$true)][string]$Destination)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
$destinationRoot = [IO.Path]::GetFullPath($Destination)
if (-not $destinationRoot.StartsWith($root + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'CRT staging must stay within the workspace.' }
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) { throw 'Visual Studio Build Tools with the C++ redistributable component is required on the build machine.' }
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $installation) { throw 'MSVC build tools installation not found.' }
$redistribution = Join-Path $installation 'VC/Redist/MSVC'
$versions = @(Get-ChildItem -LiteralPath $redistribution -Directory | Where-Object { $_.Name -match '^\d+\.\d+\.\d+$' } | Sort-Object { [version]$_.Name } -Descending)
$crt = $null
foreach ($version in $versions) {
    $candidate = Join-Path $version.FullName 'x64/Microsoft.VC143.CRT'
    if (Test-Path -LiteralPath (Join-Path $candidate 'vcruntime140.dll')) { $crt = $candidate; break }
}
if (-not $crt) { throw 'Install the Microsoft Visual C++ x64 redistributable component in Visual Studio Installer on the build machine.' }
New-Item -ItemType Directory -Force -Path $destinationRoot | Out-Null
$files = @()
foreach ($dll in @(Get-ChildItem -LiteralPath $crt -Filter '*.dll' -File)) {
    $signature = Get-AuthenticodeSignature -LiteralPath $dll.FullName
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') { throw ('CRT file lacks a valid Microsoft signature: ' + $dll.FullName) }
    $target = Join-Path $destinationRoot $dll.Name
    Copy-Item -LiteralPath $dll.FullName -Destination $target -Force
    $files += [ordered]@{file=$dll.Name;version=$dll.VersionInfo.FileVersion;sha256=(Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant()}
}
$record = [ordered]@{
    deployment='Microsoft-supported application-local deployment';
    source='Visual Studio VC/Redist/MSVC/x64/Microsoft.VC143.CRT (not System32)';
    documentation='https://learn.microsoft.com/en-us/cpp/windows/walkthrough-deploying-a-visual-cpp-application-to-an-application-local-folder';
    license='Microsoft Visual Studio distributable-code terms; these runtime files are not licensed under the application GPL';
    files=$files
}
$record | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destinationRoot 'VC_RUNTIME.json') -Encoding UTF8
$notice = @'
Microsoft Visual C++ Runtime

Copyright Microsoft Corporation. All rights reserved.
These Microsoft-signed runtime files are copied from the x64 Microsoft.VC143.CRT
redistribution directory supplied with Visual Studio Build Tools. They are
distributed under Microsoft's applicable distributable-code terms, separately
from iMirror's GPL license. No Apple binaries or System32 runtime copies are used.

Supported deployment documentation:
https://learn.microsoft.com/en-us/cpp/windows/walkthrough-deploying-a-visual-cpp-application-to-an-application-local-folder
Visual Studio redistribution list and terms:
https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution
https://visualstudio.microsoft.com/license-terms/

See VC_RUNTIME.json for file versions and SHA-256 hashes. Runtime security updates
must be included in subsequent iMirror releases when using app-local deployment.
'@
$notice | Set-Content -LiteralPath (Join-Path $destinationRoot 'MICROSOFT_RUNTIME_NOTICE.txt') -Encoding UTF8
Write-Output ('Staged official x64 Microsoft CRT files: ' + $files.Count)
