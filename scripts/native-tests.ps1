Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$build = Join-Path $root 'target/native-tests'
cmake -S (Join-Path $root 'vendor/iphone-mirror') -B $build -G 'Visual Studio 17 2022' -A x64
if ($LASTEXITCODE -ne 0) { throw 'Native configure failed' }
cmake --build $build --config Release --parallel 4
if ($LASTEXITCODE -ne 0) { throw 'Native test build failed' }
Copy-Item (Join-Path $root 'vendor/iphone-mirror/third_party/libusb-win32/bin/x64/libusb0.dll') (Join-Path $build 'src/Core/Release/') -Force
ctest --test-dir $build -C Release --output-on-failure
if ($LASTEXITCODE -ne 0) { throw 'Native tests failed' }
