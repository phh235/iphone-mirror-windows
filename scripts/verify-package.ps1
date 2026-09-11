param([string]$PackageDirectory, [switch]$InstallLifecycle)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if(-not $PackageDirectory){$PackageDirectory=(Get-Content (Join-Path $root 'dist/latest-stage.txt') -Raw).Trim()}
$package=[IO.Path]::GetFullPath($PackageDirectory)
$manifest=Get-Content (Join-Path $package 'files.sha256.json') -Raw | ConvertFrom-Json
foreach($entry in $manifest){
    $file=[IO.Path]::GetFullPath((Join-Path $package $entry.path))
    if(-not $file.StartsWith($package+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Invalid manifest path'}
    if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256){throw ("File hash mismatch: "+$entry.path)}
}
$oldPath=$env:PATH
try {
    $env:PATH=$env:SystemRoot+'\System32;'+$env:SystemRoot
    foreach($mode in @('--ui-smoke-test','--airplay-smoke-test')){
        $process=Start-Process -FilePath (Join-Path $package 'iMirror.exe') -ArgumentList $mode -WindowStyle Hidden -PassThru
        if(-not $process.WaitForExit(30000)){throw ("Smoke check timed out: "+$mode)}
        if($process.ExitCode -ne 0){throw ("Smoke check failed: "+$mode+" exit "+$process.ExitCode)}
        Write-Output ($mode+' passed')
    }
} finally {$env:PATH=$oldPath}
Write-Output ("Package hashes and isolated-PATH startup passed: "+$manifest.Count+" files.")
if($InstallLifecycle){
    $msi=Join-Path $root 'dist/iMirror-0.1.0-x64.msi'
    $install=Join-Path $env:LOCALAPPDATA 'Programs/iMirror'
    if(Test-Path $install){throw 'Refusing to replace an existing iMirror installation during a test.'}
    function Invoke-Msi([string]$Action,[string]$LogName){
        $log=Join-Path $root ('work/'+$LogName)
        $process=Start-Process -FilePath msiexec.exe -ArgumentList @($Action,('"'+$msi+'"'),'/qn','/norestart','/l*v',('"'+$log+'"')) -WindowStyle Hidden -PassThru
        if(-not $process.WaitForExit(180000)){throw 'Windows Installer timed out'}
        if($process.ExitCode -notin @(0,3010)){throw ("Windows Installer failed: "+$process.ExitCode+"; "+$log)}
    }
    Invoke-Msi '/i' 'msi-final-install.log'
    & $PSCommandPath -PackageDirectory $install
    Invoke-Msi '/x' 'msi-final-uninstall.log'
    if(Test-Path $install){throw 'Application directory remained after uninstall'}
    Invoke-Msi '/i' 'msi-final-reinstall.log'
    & $PSCommandPath -PackageDirectory $install
    Invoke-Msi '/x' 'msi-final-uninstall-again.log'
    if(Test-Path $install){throw 'Application directory remained after the second uninstall'}
    Write-Output 'Development-host MSI install/start/uninstall/reinstall/start/uninstall passed.'
}
