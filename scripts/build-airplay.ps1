Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
python (Join-Path $PSScriptRoot 'build-airplay.py')
if ($LASTEXITCODE -ne 0) { throw 'Pinned AirPlay build failed' }
& (Join-Path $PSScriptRoot 'prepare-airplay.ps1')
