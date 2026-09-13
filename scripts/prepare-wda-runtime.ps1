#Requires -Version 5.1
param([Parameter(Mandatory=$true)][string]$Destination)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$root=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$work=Join-Path $root 'work/wda-runtime-build'
New-Item -ItemType Directory -Path $work,$Destination -Force|Out-Null
$Destination=(Resolve-Path -LiteralPath $Destination).Path
function Download([string]$Url,[string]$File,[string]$Hash){
 if(!(Test-Path -LiteralPath $File) -or (Get-FileHash -LiteralPath $File).Hash -ne $Hash){
  & curl.exe --fail --location --silent --show-error --retry 2 --output $File $Url
  if($LASTEXITCODE -ne 0){throw 'WDA dependency download failed.'}
  if((Get-FileHash -LiteralPath $File).Hash -ne $Hash){throw 'WDA dependency checksum mismatch.'}
 }
}
$sdkArchive=Join-Path $work 'go1.26.8.windows-amd64.zip'
Download 'https://go.dev/dl/go1.26.8.windows-amd64.zip' $sdkArchive 'B92C3B2ADAE85A11BA71FE7216DAF0D84E82AF4C8AB6C5625807F28622043A59'
$go=Join-Path $work 'go/bin/go.exe'
if(!(Test-Path -LiteralPath $go)){Expand-Archive -LiteralPath $sdkArchive -DestinationPath $work}
$iosArchive=Join-Path $work 'go-ios-win.zip'
Download 'https://github.com/danielpaulus/go-ios/releases/download/v1.3.2/go-ios-win.zip' $iosArchive '939C6BCAAFED183A92AFB9F79CC11B1F935FA6389BFC94D3902E3F52C4DFF3FE'
$iosExtract=Join-Path $work 'go-ios'
if(!(Test-Path -LiteralPath (Join-Path $iosExtract 'ios.exe'))){Expand-Archive -LiteralPath $iosArchive -DestinationPath $iosExtract -Force}
$ios=Join-Path $iosExtract 'ios.exe'
if((Get-FileHash -LiteralPath $ios).Hash -ne 'C99B04F1D615FA716637EFAE457D5C554F32259F9D249375DE0086E3CC1A1DF5'){throw 'Unexpected go-ios executable.'}
$env:GOTOOLCHAIN='local';$env:GOWORK='off';$env:CGO_ENABLED='0';$env:GOOS='windows';$env:GOARCH='amd64'
$env:GOMODCACHE=Join-Path $work 'modules';$env:GOCACHE=Join-Path $work 'cache'
Push-Location (Join-Path $root 'vendor/wda-forwarder')
try{
 & $go test -mod=readonly ./...
 if($LASTEXITCODE -ne 0){throw 'WDA forwarder tests failed.'}
 & $go build -mod=readonly -trimpath -ldflags '-s -w' -o (Join-Path $Destination 'wda-forwarder.exe') .
 if($LASTEXITCODE -ne 0){throw 'WDA forwarder build failed.'}
 $modules=@(& $go list -mod=readonly -deps -f '{{with .Module}}{{.Path}}|{{.Version}}|{{.Dir}}{{end}}' .|Where-Object{$_}|Sort-Object -Unique)
 if($LASTEXITCODE -ne 0){throw 'Go dependency inventory failed.'}
}finally{Pop-Location}
# Use the actual downloaded binary's module build info. Its dependency versions
# differ from the source tag; do not pretend a source graph describes this EXE.
$buildInfo=@(& $go version -m $ios)
if($LASTEXITCODE -ne 0){throw 'Could not inspect go-ios binary build metadata.'}
foreach($line in $buildInfo){
 if($line -match '^\s+dep\s+(\S+)\s+(\S+)\s+(h1:\S+)'){
  $modulePath=$Matches[1];$moduleVersion=$Matches[2];$sum=$Matches[3]
  $metadata=@(& $go mod download -json ($modulePath+'@'+$moduleVersion))
  if($LASTEXITCODE -ne 0){throw ('Could not retrieve dependency license: '+$modulePath)}
  $entry=($metadata -join "`n")|ConvertFrom-Json
  if($entry.Sum -ne $sum){throw ('Binary dependency checksum mismatch: '+$modulePath)}
  $modules+=($modulePath+'|'+$moduleVersion+'|'+$entry.Dir)
 }
}
$modules=@($modules|Sort-Object -Unique)
Copy-Item -LiteralPath $ios -Destination (Join-Path $Destination 'ios.exe') -Force
$licenses=Join-Path $Destination 'licenses'
New-Item -ItemType Directory -Path $licenses -Force|Out-Null
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $licenses 'iMirror-GPL-3.0.txt') -Force
Copy-Item -LiteralPath (Join-Path $work 'go/LICENSE') -Destination (Join-Path $licenses 'Go-LICENSE.txt') -Force
$inventory=@()
foreach($module in $modules){
 $fields=$module -split '\|',3
 if($fields.Count -ne 3 -or !$fields[2] -or $fields[0] -eq 'imirror.local/wda-forwarder'){continue}
 if($fields[0] -eq 'github.com/danielpaulus/go-ios' -and !$fields[1]){$fields[1]='v1.3.2'}
 $files=@(Get-ChildItem -LiteralPath $fields[2] -File|Where-Object{$_.Name -match '^(LICENSE|COPYING)(\.|$)'})
 if(!$files.Count){throw ('Missing Go dependency license: '+$fields[0])}
 $prefix=($fields[0]+'-'+$fields[1]) -replace '[^A-Za-z0-9._-]','_'
 foreach($file in $files){Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $licenses ($prefix+'-'+$file.Name)) -Force}
 $inventory += [ordered]@{module=$fields[0];version=$fields[1]}
}
[IO.File]::WriteAllText((Join-Path $Destination 'runtime-manifest.json'),([ordered]@{go_version='1.26.8';go_ios='1.3.2 official release asset';go_ios_build_info=@($buildInfo|Select-Object -Skip 1);go_ios_source_limit='Official asset reports a modified upstream revision; it is hash-pinned, not claimed rebuilt from the tag.';ios_sha256=(Get-FileHash -LiteralPath (Join-Path $Destination 'ios.exe')).Hash;forwarder_sha256=(Get-FileHash -LiteralPath (Join-Path $Destination 'wda-forwarder.exe')).Hash;modules=$inventory;scope='Native runtime only; no Apple binaries, signing keys or phone pairing data'}|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
Write-Output ('Prepared native WDA runtime: '+$Destination)
