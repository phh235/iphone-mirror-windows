param([switch]$SkipChecks, [switch]$SkipAirPlayBuild, [switch]$SkipSourcePackage)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
        & $Executable @Arguments
        if ($LASTEXITCODE -ne 0) { throw "$Executable failed with exit code $LASTEXITCODE" }
    }
    if (-not $SkipChecks) {
        Invoke-Checked cargo @('fmt','--all','--','--check')
        Invoke-Checked cargo @('clippy','--workspace','--all-targets','--all-features','--locked','--','-D','warnings')
        Invoke-Checked cargo @('test','--workspace','--all-targets','--locked')
        & (Join-Path $PSScriptRoot 'native-tests.ps1')
    }
    if (-not $SkipAirPlayBuild) { & (Join-Path $PSScriptRoot 'build-airplay.ps1') }
    Invoke-Checked cargo @('build','--release','--locked')
    $version = '0.1.0'
    # Each invocation gets a fresh staging tree; no stale files are harvested.
    $build = Join-Path $root ('work/package-' + [Guid]::NewGuid().ToString('N'))
    $stage = Join-Path $build 'iMirror'
    $dist = Join-Path $root 'dist'
    New-Item -ItemType Directory -Force $stage,$dist | Out-Null
    Copy-Item -LiteralPath (Join-Path $root 'target/release/iMirror.exe'),(Join-Path $root 'target/release/libusb-1.0.dll'),(Join-Path $root 'target/release/libusb0.dll'),(Join-Path $root 'target/release/iPhoneMirror.UsbConfigurationSwitch.exe') -Destination $stage
    & (Join-Path $PSScriptRoot 'prepare-vc-runtime.ps1') -Destination $stage
    & (Join-Path $PSScriptRoot 'prepare-airplay.ps1') -Destination (Join-Path $stage 'AirPlay')
    $noticeDir = Join-Path $stage 'licenses'
    Invoke-Checked python @((Join-Path $PSScriptRoot 'runtime-inventory.py'),'--runtime',(Join-Path $stage 'AirPlay'),'--output',$noticeDir)
    Copy-Item -LiteralPath (Join-Path $noticeDir 'THIRD_PARTY_LICENSES.md') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $root 'LICENSE'),(Join-Path $root 'README.md'),(Join-Path $root 'HUONG_DAN.md'),(Join-Path $root 'docs/VALIDATION.md') -Destination $stage
    New-Item -ItemType Directory -Force (Join-Path $stage 'docs') | Out-Null
    Copy-Item -LiteralPath (Join-Path $root 'docs/VALIDATION.md') -Destination (Join-Path $stage 'docs')
    Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object { $_.LastWriteTime.Year -lt 1980 } | ForEach-Object { $_.LastWriteTime = [DateTime]'1980-01-01' }
    $manifest = @(Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
        [ordered]@{path=$_.FullName.Substring($stage.Length+1).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();bytes=$_.Length}
    })
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $stage 'files.sha256.json') -Encoding utf8
    $wix = Join-Path $root 'work/dependencies/wix314'
    if (-not (Test-Path (Join-Path $wix 'candle.exe'))) {
        $archive = Join-Path $root 'work/dependencies/wix314-binaries.zip'
        Invoke-Checked curl.exe @('--fail','--location','--silent','--show-error','--output',$archive,'https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip')
        if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne '6AC824E1642D6F7277D0ED7EA09411A508F6116BA6FAE0AA5F2C7DAA2FF43D31') { throw 'WiX archive hash mismatch' }
        Expand-Archive -LiteralPath $archive -DestinationPath $wix -Force
    }
    $wxs = Join-Path $build 'Product.wxs'
    Invoke-Checked python @((Join-Path $PSScriptRoot 'generate-installer.py'),'--stage',$stage,'--output',$wxs,'--version',$version)
    $msi = Join-Path $dist "iMirror-$version-x64.msi"
    $setup = Join-Path $dist "iMirror-$version-Setup.exe"
    Invoke-Checked (Join-Path $wix 'candle.exe') @('-nologo','-arch','x64','-ext','WixUIExtension','-out',(Join-Path $build 'Product.wixobj'),$wxs)
    Invoke-Checked (Join-Path $wix 'light.exe') @('-nologo','-sice:ICE91','-wx','-ext','WixUIExtension','-out',$msi,(Join-Path $build 'Product.wixobj'))
    Invoke-Checked (Join-Path $wix 'candle.exe') @('-nologo','-ext','WixBalExtension',"-dProductVersion=$version","-dMsiPath=$msi",("-dLicensePath="+(Join-Path $root 'installer/license.rtf')),'-out',(Join-Path $build 'Bundle.wixobj'),(Join-Path $root 'installer/Bundle.wxs'))
    Invoke-Checked (Join-Path $wix 'light.exe') @('-nologo','-ext','WixBalExtension','-out',$setup,(Join-Path $build 'Bundle.wixobj'))
    $portable = Join-Path $dist "iMirror-$version-x64.zip"
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $portable -Force
    $stage | Set-Content -LiteralPath (Join-Path $dist 'latest-stage.txt')
    $artifacts = @($msi,$setup,$portable)
    if (-not $SkipSourcePackage) {
        Invoke-Checked python @((Join-Path $PSScriptRoot 'package-source.py'))
        $artifacts += Join-Path $dist 'iMirror-0.1.0-source.zip'
    }
    $lines = foreach ($artifact in $artifacts) { (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($artifact) }
    $lines | Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Encoding ascii
    Write-Output "Built MSI, Setup EXE and portable archive in $dist"
    Write-Output "Executable: $(Join-Path $stage 'iMirror.exe')"
    Write-Output 'These are engineering artifacts. Consult VALIDATION.md before claiming production readiness.'
} finally { Pop-Location }
