param([ValidateRange(1,3600)][int]$Seconds=1800,[string]$Executable)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if(-not $Executable){$Executable=Join-Path $root 'target/release/iMirror.exe'}
if(@(Get-Process iMirror -ErrorAction SilentlyContinue).Count){throw 'Close the existing iMirror session before a dedicated USB stress test.'}
$stamp=[DateTime]::Now.ToString('yyyyMMdd-HHmmss')
$run=Join-Path $root ('work/stress-'+$stamp)
New-Item -ItemType Directory $run | Out-Null
$source=Split-Path ([IO.Path]::GetFullPath($Executable)) -Parent
foreach($name in @('iMirror.exe','libusb0.dll','libusb-1.0.dll','iPhoneMirror.UsbConfigurationSwitch.exe')){
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination $run
}
$report=Join-Path $root ('benchmarks/usb-stress-'+$stamp+'.json')
$resources=Join-Path $root ('benchmarks/usb-stress-'+$stamp+'-resources.jsonl')
$process=Start-Process -FilePath (Join-Path $run 'iMirror.exe') -ArgumentList @('--benchmark','usb','--render','--seconds',$Seconds,'--output',('"'+$report+'"')) -WindowStyle Normal -PassThru
[ordered]@{pid=$process.Id;binary=(Join-Path $run 'iMirror.exe');sha256=(Get-FileHash (Join-Path $run 'iMirror.exe') -Algorithm SHA256).Hash;started=[DateTime]::UtcNow.ToString('o');seconds=$Seconds;report=$report;resources=$resources} |
    ConvertTo-Json | Set-Content (Join-Path $root 'work/active-stress.json') -Encoding utf8
Write-Output ("USB stress run started: PID "+$process.Id+"; duration "+$Seconds+" seconds")
$started=[DateTime]::UtcNow
while(-not $process.HasExited){
    $process.Refresh()
    if($process.HasExited){break}
    [ordered]@{elapsed=([DateTime]::UtcNow-$started).TotalSeconds;cpu_seconds=$process.TotalProcessorTime.TotalSeconds;working_set=$process.WorkingSet64;private_bytes=$process.PrivateMemorySize64;handles=$process.HandleCount} |
        ConvertTo-Json -Compress | Add-Content -LiteralPath $resources -Encoding utf8
    Start-Sleep -Seconds 5
}
Write-Output ("Benchmark exit: "+$process.ExitCode)
Get-Content $report -Raw | ConvertFrom-Json |
    Select-Object elapsed_seconds,source_frames,source_fps_measured,render_fps_measured,render_submitted_frames,passed_capture_check,error
if($process.ExitCode -ne 0){throw ("USB stress test did not complete: "+$report)}
