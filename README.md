# iMirror

A native Windows application for iPhone mirroring. The Rust application uses a
pinned C++ QuickTime/Media Foundation/D3D11 engine for USB video and a
managed UxPlay receiver for AirPlay. BLE HID and optional WebDriverAgent provide
separate control paths.

**Engineering preview:** input, sustained performance, and clean-machine
installation have not passed their release gates. See [validation](docs/VALIDATION.md).
Real USB video has been received and rendered on an iPhone15,4 running iOS 27.0.
The user confirmed smooth picture and correct orientation. Broader validation is pending.

## Install and launch

The current local portable build is `dist/portable/iMirror.exe`. Keep the entire
portable folder, including its USB DLLs, helper, licenses and `AirPlay` directory.
Generated installers and source archives can be recreated by the release script;
obsolete packages were removed during cleanup. For installer releases, use the
MSI or Setup EXE from that release.
Installation is per user under `%LOCALAPPDATA%\Programs\iMirror`.
The installed application needs no Rust, Visual Studio, Python, Node.js, CMake
or FFmpeg command-line tools. These binaries are currently unsigned.

The MSI creates a Start Menu shortcut. Its feature selection offers an optional
Desktop shortcut. Uninstall through Windows Installed Apps or the Start Menu
entry.

Silent install: `msiexec /i iMirror-0.1.0-x64.msi /qn /norestart`.
Silent uninstall: `msiexec /x iMirror-0.1.0-x64.msi /qn /norestart`.
The Setup EXE supports `/quiet /norestart` and `/uninstall /quiet /norestart`.

## Connect

USB: install Apple's supported device software, connect and unlock the iPhone,
and tap **Trust This Computer**. Select the device and Connect.
The QuickTime backend needs a compatible Apple/USB driver stack. Some setups
require a device-specific capture filter; this host has captured real video
without this continuation installing a driver. iMirror does not silently install one.
Do not replace the Apple USB parent driver with WinUSB. Driver compatibility varies across machines.

AirPlay: put Windows and the iPhone on the same local network, click AirPlay,
then select iMirror in the iPhone's Control Center > Screen Mirroring.
The receiver runs only after that explicit action. If Windows asks about network
access, allow the intended private network yourself. No blanket firewall rule is
installed. Built-in mDNS discovery avoids requiring the Bonjour service.

BLE mouse: click **BLE mouse**, pair the advertised computer in iPhone Bluetooth
settings, enable AssistiveTouch if needed, and select the subscribed phone.
The Windows adapter must support BLE peripheral mode. Pairing, HID subscription
and relative pointer motion were confirmed on the tested iPhone; broader device
compatibility and sustained reliability remain unvalidated.

Mouse speed is adjustable in the **BLE control** window (5-200%, default 25%).
Changes apply immediately and are saved automatically. 100% restores the previous
unscaled motion. This affects physical mouse movement; diagnostic button steps
remain fixed. iOS tracking acceleration can still differ from Windows.

Audio is disabled: no Mute control, USB PCM playback worker, or AirPlay audio
receiver pipeline. USB protocol liveness handling is retained.

WDA control: supply a properly signed, installed WebDriverAgent runner and a local
tunnel/port forward at `http://127.0.0.1:8100`, then click WDA control.
Apple signing and Developer Mode requirements apply. iMirror cannot generate
signing credentials. WDA is optional for mirroring.

Click the video to capture input; Escape releases it.
Ctrl+Shift+F toggles fullscreen while the video is focused.
Source FPS counts received source timestamps. Decode time is local processing
time, not end-to-end latency. No 60 FPS, resolution or latency guarantee is made.

## Build

On Windows x64, install Rust MSVC, Visual Studio 2022 C++ Build Tools with a
Windows SDK, CMake and Python 3.11+ (build tools only). Then run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1
```

The build downloads hash-pinned MSYS packages (about 420 MiB with the base),
rebuilds UxPlay and minimal AAC/ALAC libraries, runs Rust and C++ checks, and
creates an MSI, Setup EXE and portable ZIP. Downloaded build dependencies stay
under `work`; they are not installed into the system toolchain.

For an incremental package using an already built AirPlay runtime:
`powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1 -SkipAirPlayBuild`.
`-SkipChecks` is for local packaging iteration and does not satisfy release checks.

## License and source

GPL-3.0-only. Upstream copyright and license notices are bundled in `licenses`.
See the generated `THIRD_PARTY_LICENSES.md`, native source lock files, and
`vendor/iphone-mirror/PATCHES.md`. LGPL runtime libraries remain replaceable.
The source ZIP contains the native source archives, Cargo dependencies and build recipes.

## Settings and benchmarking

Settings stores the receiver name, quality profile, VSync and reconnect preference
per user. Apply restarts an active AirPlay receiver when its name or source profile
changes. USB quality limits affect local rendering. Fit/1:1 and Rotate also update
input coordinate mapping.

A bounded real-device benchmark is available:
`iMirror.exe --benchmark usb --render --seconds 1800 --output usb-test.json`.
Use `airplay` instead of `usb` for the wireless receiver. The full sample stream
goes to an adjacent .samples.jsonl; .progress.json updates during the run.
Keep the phone awake for a continuous test. Closing the diagnostic window
cancels the benchmark and saves a failed/cancelled result. No test reports
physical display or end-to-end latency merely because Present accepted frames.


## Repository size and cleanup

Only source, pinned native components, licenses, manifests and build recipes
belong in Git. `target/`, `work/`, `dist/`, `research/` and generated benchmark
reports are ignored. Raw hardware evidence is kept locally, not pushed.

Build caches can occupy several GiB because they contain Rust debug metadata,
Windows bindings, MSYS tools and downloaded corresponding-source archives.
Keep the latest staged build under `dist/` and preview cleanup with:

```powershell
pwsh -File scripts/clean.ps1 -Preview
pwsh -File scripts/clean.ps1
```

This requires PowerShell 7, validates cleanup targets, refuses in-use paths and
preserves source, `.git`, rollback refs, research and small validation records.
It selects the newest staged EXE with a matching `BUILD_MANIFEST.json`; use
`-KeepRelease dist/ui-logo-20260912` to choose a specific build. Before removing
bulky screenshots and old test binaries, it creates a local evidence ZIP and
verifies every archived file with SHA-256. Older `dist` copies are removed.
The next build regenerates Cargo outputs
and downloads the hash-pinned native toolchain/source archives as needed.
