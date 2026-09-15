@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\configure-wda.ps1"
if errorlevel 1 echo WDA registration did not complete. See the message above.
pause
