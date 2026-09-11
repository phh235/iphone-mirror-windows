param([Parameter(Mandatory=$true)][string]$MsiPath,[switch]$InsideCleanMachine)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if(-not $InsideCleanMachine){throw 'Run this script in a clean Windows VM/Sandbox with -InsideCleanMachine. It does not certify this development PC.'}
$found=@(Get-Command cargo,rustc,git,node,cmake,ffmpeg,cl -ErrorAction SilentlyContinue)
if($found.Count){throw ('Clean-machine precondition failed; developer tools found: '+($found.Name -join ', '))}
$msi=[IO.Path]::GetFullPath($MsiPath)
$install=Join-Path $env:LOCALAPPDATA 'Programs/iMirror'
if(Test-Path $install){throw 'The clean test requires no existing iMirror installation'}
function Msi([string]$Action,[string]$Phase){
    $log=Join-Path $env:TEMP ('iMirror-clean-'+$Phase+'.log')
    $p=Start-Process msiexec.exe -ArgumentList @($Action,('"'+$msi+'"'),'/qn','/norestart','/l*v',('"'+$log+'"')) -WindowStyle Hidden -PassThru
    if(-not $p.WaitForExit(180000)){throw 'MSI timeout'}
    if($p.ExitCode -notin @(0,3010)){throw ("MSI failed: "+$p.ExitCode+"; log="+$log)}
}
for($cycle=1;$cycle -le 2;$cycle++){
    Msi '/i' ('install-'+$cycle)
    $p=Start-Process (Join-Path $install 'iMirror.exe') -ArgumentList '--ui-smoke-test' -WindowStyle Hidden -PassThru
    if(-not $p.WaitForExit(30000) -or $p.ExitCode -ne 0){throw 'Installed native app did not launch/exit successfully'}
    Msi '/x' ('uninstall-'+$cycle)
    if(Test-Path $install){throw 'Uninstall left the application directory'}
}
Write-Output 'Two clean-environment install/start/uninstall cycles passed on the environment selected by the operator.'
Write-Output 'Record the Windows image, account privileges, installed software inventory and these logs with the result.'
