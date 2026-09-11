param([string]$Executable,[string]$Output)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if(-not $Executable){$Executable=Join-Path $env:LOCALAPPDATA 'Programs/iMirror/iMirror.exe'}
if(-not (Test-Path $Executable)){$Executable=Join-Path $root 'target/release/iMirror.exe'}
if(-not $Output){$Output=Join-Path $root 'work/host-diagnostics.json'}
$p=Start-Process -FilePath $Executable -ArgumentList @('--diagnostics','--output',('"'+[IO.Path]::GetFullPath($Output)+'"')) -WindowStyle Hidden -PassThru
if(-not $p.WaitForExit(30000) -or $p.ExitCode -ne 0){throw 'Diagnostics did not complete successfully'}
Get-Content -LiteralPath $Output -Raw
