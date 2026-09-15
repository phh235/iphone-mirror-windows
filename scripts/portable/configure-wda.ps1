#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
try {
    if (Get-Process iMirror -ErrorAction SilentlyContinue) { throw 'Close iMirror before setup.' }
    Write-Output 'Optional WDA registration. Read the WDA guide first.'
    Write-Output 'No Apple password is needed here. This does not sign or install a runner.'
    $deviceId = (Read-Host 'Your device identifier (from WDA/ios.exe list)').Trim()
    $bundleId = (Read-Host 'Installed signed runner bundle identifier').Trim()
    $imagePath = (Read-Host 'Full local path to the Apple image Restore directory').Trim().Trim('"')
    & (Join-Path $PSScriptRoot 'register-wda-runtime.ps1') -DeviceId $deviceId -RunnerBundleId $bundleId -DeveloperImagePath $imagePath
    Write-Output 'Open iMirror, enable Advanced features and choose WDA in Control settings.'
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
