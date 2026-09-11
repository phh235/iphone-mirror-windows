Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$source = Join-Path $root 'research/iPhoneMirror'
$destination = Join-Path $root 'vendor/iphone-mirror'
New-Item -ItemType Directory -Force $destination | Out-Null
foreach ($relative in @('src/Core','third_party/libusb','third_party/libusb-win32')) {
    $target = Join-Path $destination (Split-Path $relative -Parent)
    New-Item -ItemType Directory -Force $target | Out-Null
    Copy-Item -LiteralPath (Join-Path $source $relative) -Destination $target -Recurse -Force
}
$headers = Join-Path $destination 'src/WirelessHost'
New-Item -ItemType Directory -Force $headers | Out-Null
Copy-Item (Join-Path $source 'src/WirelessHost/*.h') $headers -Force
Copy-Item (Join-Path $source 'LICENSE') (Join-Path $destination 'LICENSE') -Force
Copy-Item (Join-Path $source 'LICENSE') (Join-Path $root 'LICENSE') -Force
$runtime = Join-Path $destination 'third_party/libusb-win32/bin/x64'
New-Item -ItemType Directory -Force $runtime | Out-Null
Copy-Item (Join-Path $source 'src/DriverInstaller/Assets/libusb-win32-1.2.6.0/amd64/libusb0.dll') $runtime -Force
$records = @(Get-ChildItem (Join-Path $root 'research') -Directory | ForEach-Object {
    $repo = $_.FullName
    [ordered]@{ name = $_.Name; url = (& git -C $repo remote get-url origin); commit = (& git -C $repo rev-parse HEAD); last_commit = (& git -C $repo log -1 --format='%cI %s') }
})
$records | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'docs/upstream-lock.json') -Encoding utf8
