#Requires -Version 5.1
param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Fa-f0-9-]{20,80}$')][string]$DeviceId,
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z][A-Za-z0-9._-]{2,254}$')][string]$RunnerBundleId,
    [string]$PairingSource
)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if(!$RunnerBundleId.Contains('.')){throw 'Expected an installed application bundle identifier.'}
if(Get-Process iMirror -ErrorAction SilentlyContinue){throw 'Close iMirror before registering its private WDA setup.'}
$data=Join-Path $env:LOCALAPPDATA 'iMirror/wda'
$pairing=Join-Path $data 'pairing'
New-Item -ItemType Directory -Path $pairing -Force|Out-Null
if($PairingSource){
    $source=(Resolve-Path -LiteralPath $PairingSource).Path
    $items=@(Get-Item -LiteralPath $source -Force)+@(Get-ChildItem -LiteralPath $source -Recurse -Force)
    if(@($items|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Do not import pairing data through filesystem links.'}
    Get-ChildItem -LiteralPath $source -Force|Copy-Item -Destination $pairing -Recurse -Force
}
$path=Join-Path $data 'setup.json'
$temporary=Join-Path $data ('setup-'+[Guid]::NewGuid().ToString('N')+'.tmp')
$value=[ordered]@{version=1;device_id=$DeviceId;runner_bundle_id=$RunnerBundleId}
[IO.File]::WriteAllText($temporary,($value|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
if(Test-Path -LiteralPath $path){[IO.File]::Replace($temporary,$path,($path+'.bak'))}else{[IO.File]::Move($temporary,$path)}
Write-Output 'Registered private WDA setup. No app was signed or installed; no credentials were read or printed.'
