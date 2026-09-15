#Requires -Version 5.1
param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Fa-f0-9-]{20,80}$')][string]$DeviceId,
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z][A-Za-z0-9._-]{2,254}$')][string]$RunnerBundleId,
    [string]$PairingSource,
    [string]$DeveloperImagePath
)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if(!$RunnerBundleId.Contains('.')){throw 'Expected an installed application bundle identifier.'}
if(Get-Process iMirror -ErrorAction SilentlyContinue){throw 'Close iMirror before registering its private WDA setup.'}
$data=Join-Path $env:LOCALAPPDATA 'iMirror/wda'
$pairing=Join-Path $data 'pairing'
$path=Join-Path $data 'setup.json'
$existing=$null
if(Test-Path -LiteralPath $path){$existing=Get-Content -LiteralPath $path -Raw|ConvertFrom-Json}
$image=$null
if($PSBoundParameters.ContainsKey('DeveloperImagePath')){
    if([string]::IsNullOrWhiteSpace($DeveloperImagePath)){throw 'Provide the Apple developer image Restore directory.'}
    $image=(Resolve-Path -LiteralPath $DeveloperImagePath).Path
    if($image -notmatch '^[A-Za-z]:\\' -or !(Test-Path -LiteralPath $image -PathType Container) -or !(Test-Path -LiteralPath (Join-Path $image 'BuildManifest.plist') -PathType Leaf)){
        throw 'Developer image must be a local Restore directory containing BuildManifest.plist.'
    }
    $cursor=Get-Item -LiteralPath $image -Force
    while($null -ne $cursor){
        if($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Do not register a developer image through filesystem links.'}
        $cursor=$cursor.Parent
    }
    if(@(Get-ChildItem -LiteralPath $image -Recurse -Force|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Developer image contains filesystem links.'}
}elseif($null -ne $existing -and $existing.device_id -eq $DeviceId -and $null -ne $existing.PSObject.Properties['developer_image_restore']){
    $image=$existing.developer_image_restore
}
New-Item -ItemType Directory -Path $pairing -Force|Out-Null
if($PairingSource){
    $source=(Resolve-Path -LiteralPath $PairingSource).Path
    $items=@(Get-Item -LiteralPath $source -Force)+@(Get-ChildItem -LiteralPath $source -Recurse -Force)
    if(@($items|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Do not import pairing data through filesystem links.'}
    Get-ChildItem -LiteralPath $source -Force|Copy-Item -Destination $pairing -Recurse -Force
}
$temporary=Join-Path $data ('setup-'+[Guid]::NewGuid().ToString('N')+'.tmp')
$value=[ordered]@{version=1;device_id=$DeviceId;runner_bundle_id=$RunnerBundleId}
if($image){$value.developer_image_restore=$image}
[IO.File]::WriteAllText($temporary,($value|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
if(Test-Path -LiteralPath $path){[IO.File]::Replace($temporary,$path,($path+'.bak'))}else{[IO.File]::Move($temporary,$path)}
Write-Output 'Registered private WDA setup. No app was signed or installed; no credentials were read or printed.'
if($image){Write-Output 'Cached developer image registered for automatic WDA preparation. It was not copied into the app package.'}
