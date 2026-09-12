> Historical snapshot; see [current validation](../VALIDATION.md) and the [history index](README.md). This record is not current setup guidance.

You are a principal Windows systems engineer and Rust performance engineer.

Your task is NOT to produce a demo, architecture proposal, proof of concept, mockup, or partially-working repository.

Your task is to build a production-grade Windows desktop application that mirrors and controls a real iPhone from Windows, with an experience as close as technically possible to scrcpy / Apple's iPhone Mirroring.

You must continue implementing, compiling, testing, debugging, packaging, and validating until the project produces a real installable Windows release.

======================================================================
PRODUCT GOAL
======================================================================

Build a native Windows application named:

iMirror

Core goals:

- Mirror a real iPhone to Windows.
- Control the iPhone using Windows mouse and keyboard.
- No jailbreak.
- No Electron.
- No React.
- No browser UI.
- No Python runtime.
- No Node.js runtime.
- Rust is the primary application language.
- Reuse mature open-source native components where rewriting them would reduce stability or performance.
- Windows-native GPU rendering.
- Target 60 FPS when the source device/protocol supports it.
- Excellent image quality.
- Lowest practical end-to-end latency.
- Reliable reconnect/disconnect behavior.
- Produce standalone Windows EXE.
- Produce an installer.
- A normal user must not need Rust, Git, Python, Node, CMake, FFmpeg CLI, or developer tooling after installation.

The application should feel like:

scrcpy for iPhone on Windows.

======================================================================
IMPORTANT: DO NOT MAKE FALSE GUARANTEES
======================================================================

Do not claim:

- guaranteed 60 FPS,
- guaranteed resolution,
- guaranteed latency,
- support for every Windows computer,
- support for every Bluetooth chipset,
- support for every iOS version,

unless those claims have actually been validated.

Instead:

1. detect runtime capabilities,
2. benchmark them,
3. expose actual source FPS/resolution/latency,
4. automatically select the best supported configuration,
5. gracefully fall back when necessary.

A 60 FPS UI loop is NOT proof that the iPhone source is 60 FPS.

Measure actual unique source frames.

======================================================================
SUPPORTED RELEASE TARGET
======================================================================

Primary support:

- Windows 11 x64
- Windows 10 22H2 x64 where APIs permit
- iOS 17+
- USB-C and Lightning iPhones supported where underlying protocol allows it

Optional after primary release:

- Windows ARM64

Do NOT waste initial development time supporting Windows 7/8.

======================================================================
PERFORMANCE PRIORITY
======================================================================

When making trade-offs, use this exact priority:

1. stability
2. input latency
3. video latency
4. frame pacing
5. actual 60 FPS
6. image quality
7. GPU/CPU efficiency
8. memory usage
9. application size
10. implementation simplicity

Never sacrifice stability or latency merely to save a few MB.

======================================================================
OPEN-SOURCE RESEARCH
======================================================================

Before implementing the video/control stack, inspect current upstream versions of:

- UxPlay
- quicktime_video_hack / QVH
- go-ios
- WebDriverAgent
- windows-ble-hid
- ioscpy / ioscpy-windows
- pcairplay
- libimobiledevice
- any newer actively-maintained equivalent discovered during research

Audit each repository for:

- license
- language
- release activity
- last meaningful commit
- Windows support
- iOS support
- video transport
- maximum known FPS
- hardware decode compatibility
- control mechanism
- jailbreak requirement
- external runtime dependencies
- redistribution restrictions

DO NOT choose a repository because it is easiest to integrate.

Choose based on:

- maturity
- stability
- latency
- maintainability
- licensing
- Windows compatibility

Do not use Salinator513/iPhone-on-Windows as the application's foundation.

It may only be inspected as a reference implementation for WDA input concepts.

======================================================================
ARCHITECTURE
======================================================================

Create a Cargo workspace approximately like:

imirror/
│
├── Cargo.toml
├── Cargo.lock
│
├── crates/
│   ├── app/
│   ├── platform-windows/
│   ├── device/
│   ├── video-core/
│   ├── video-usb/
│   ├── video-airplay/
│   ├── decoder/
│   ├── renderer/
│   ├── input-core/
│   ├── input-ble/
│   ├── input-wda/
│   ├── coordinate-map/
│   ├── diagnostics/
│   └── updater/
│
├── vendor/
├── assets/
├── scripts/
├── installer/
├── tests/
└── docs/

Keep modules independently replaceable.

Do not tightly couple:
- video transport,
- decoder,
- renderer,
- control mechanism,
- UI.

Define stable Rust traits/interfaces between them.

======================================================================
WINDOWING / UI
======================================================================

Use a truly native/lightweight desktop stack.

Preferred:

- winit
- windows-rs
- egui only where useful

A WebView is forbidden.

Main window:

┌─────────────────────────────────────────┐
│ device                           stats  │
├─────────────────────────────────────────┤
│                                         │
│              iPhone video               │
│                                         │
└─────────────────────────────────────────┘

Minimal overlay controls:

- Connect
- Disconnect
- USB
- AirPlay
- BLE control
- WDA control
- FPS
- Fit
- 1:1
- Fullscreen
- Rotate
- Audio
- Statistics
- Settings

UI rendering must never block video decoding.

======================================================================
VIDEO BACKEND A: USB — PRIMARY
======================================================================

USB is the preferred low-latency transport.

Research and implement the best currently practical native USB video path.

Evaluate:

- QuickTime-compatible USB video stream
- QVH
- similar newer implementations
- Apple's device transport exposed through available open-source work

DO NOT use screenshot polling for normal operation.

Screenshot polling may exist only as:

- diagnostics,
- fallback,
- testing.

Do NOT use:

PNG -> HTTP -> browser -> image

or

JPEG -> HTTP -> browser -> image.

Do not make MJPEG the final high-performance implementation unless no encoded native stream is available.

Preferred data flow:

iPhone
  ↓
USB encoded video stream
  ↓
H264/H265 parser
  ↓
Windows hardware decoder
  ↓
GPU texture
  ↓
D3D renderer

No unnecessary re-encoding.

======================================================================
VIDEO BACKEND B: AIRPLAY
======================================================================

Provide wireless mirroring as a second backend.

Prefer mature AirPlay work such as UxPlay where appropriate.

Requirements:

- Bonjour/mDNS discovery
- reliable reconnect
- selectable receiver name
- H.264 support
- H.265 where supported
- source FPS measurement
- 30 FPS fallback
- 60 FPS request where protocol supports it
- source resolution detection

AirPlay failure must never crash the application.

======================================================================
WINDOWS VIDEO DECODING
======================================================================

Evaluate:

1. Windows Media Foundation
2. Direct3D11 Video Decoder
3. FFmpeg hardware acceleration
4. GStreamer D3D11

Preferred final design:

Media Foundation / D3D11 hardware acceleration

if it satisfies compatibility and stability requirements.

Do not ship a large FFmpeg runtime if native Windows APIs are sufficient.

But:

DO NOT choose Media Foundation merely for purity.

If another solution is materially more reliable, benchmark both and choose based on evidence.

======================================================================
RENDERER
======================================================================

Use:

- Direct3D 11 initially

Consider D3D12 only if there is a demonstrated benefit.

Rendering requirements:

- high-DPI aware
- portrait
- landscape
- resizing
- fullscreen
- multi-monitor
- 120/144/165/180 Hz Windows monitors
- vsync configurable
- correct aspect ratio
- no stretching
- minimal buffering

Implement latest-frame rendering.

Never allow frames to queue indefinitely.

Preferred queue depth:

1-3 frames maximum.

Under load:

DROP OLD VIDEO FRAMES.

Do not increase latency by preserving stale frames.

======================================================================
ZERO-COPY
======================================================================

Use zero-copy or minimal-copy GPU paths wherever practical.

Avoid:

GPU -> CPU -> GPU

for every frame.

Avoid allocating a new large buffer per frame.

Use:

- reusable buffers
- pools
- bounded channels
- preallocated structures

Measure copies in the hot path.

======================================================================
INPUT SYSTEM
======================================================================

Provide TWO control implementations.

----------------------------------------------------------------------
A. BLE HID — DEFAULT HUMAN CONTROL
----------------------------------------------------------------------

Windows should behave as a Bluetooth keyboard/mouse/HID device where compatible.

Study windows-ble-hid and relevant Windows APIs.

Support:

- mouse motion
- left click
- drag
- scroll wheel
- keyboard
- modifier keys
- optional media keys

Provide clear setup instructions for iOS AssistiveTouch where required.

Detect whether the Windows Bluetooth adapter supports required BLE peripheral functionality.

If unsupported:

DO NOT FAIL SILENTLY.

Report:

"Bluetooth adapter does not support BLE HID peripheral mode."

Allow WDA control fallback.

----------------------------------------------------------------------
B. WDA — PRECISION CONTROL
----------------------------------------------------------------------

Implement an optional WebDriverAgent controller.

Use:

- go-ios where necessary
- WebDriverAgent
- native Rust HTTP client

Do NOT embed Python.

Support:

tap(x, y)
swipe(x1, y1, x2, y2, duration)
type(text)
home
lock
volume
app switcher where supported

WDA is optional and must not be required to simply mirror the screen.

Handle:

- WDA session recreation
- device reconnect
- request timeout
- signing expiration
- tunnel reconnect

Explain Apple signing requirements honestly.

======================================================================
COORDINATE MAPPING
======================================================================

This must be implemented correctly.

Input coordinates must account for:

- iPhone native resolution
- stream resolution
- portrait
- landscape left
- landscape right
- Windows DPI scaling
- monitor scale
- title bar
- toolbar
- letterboxing
- Fit mode
- 1:1 mode
- fullscreen
- window resizing

Pipeline:

physical mouse position
↓
client coordinates
↓
remove video viewport offset
↓
remove letterbox
↓
normalize x/y to [0,1]
↓
apply rotation matrix
↓
map to device coordinate system
↓
controller

Write automated unit tests for all transforms.

======================================================================
MOUSE UX
======================================================================

Interaction:

left click
→ tap

left down + drag
→ swipe/drag

mouse wheel
→ vertical scroll

Shift + wheel
→ horizontal scroll where practical

keyboard input
→ iPhone focused input

Esc
→ release input capture

Ctrl+Shift+F
→ fullscreen

Do not trigger input when mouse is outside the actual iPhone viewport.

======================================================================
AUDIO
======================================================================

If transport exposes audio:

- decode audio
- output through WASAPI
- maintain A/V synchronization
- allow mute
- choose output device

Audio must not cause video buffers to grow uncontrollably.

======================================================================
THREADING MODEL
======================================================================

Separate critical pipelines.

Example:

Device thread
Video receive thread
Decode thread
Render thread
Input thread
Audio thread
UI thread

Do not block render loop on:

- Bluetooth
- WDA HTTP
- discovery
- filesystem
- logging
- networking

Use bounded lock-free or low-contention queues where justified.

Do not create threads per frame/request.

======================================================================
ERROR MODEL
======================================================================

Create explicit errors such as:

DeviceError
PairingError
TransportError
AirPlayError
UsbVideoError
DecodeError
RenderError
BluetoothError
WdaError
AudioError
InstallerError

Use thiserror in libraries.

anyhow is acceptable only at application boundaries.

Avoid unwrap() and expect() in production execution paths.

======================================================================
OBSERVABILITY
======================================================================

Use:

tracing
tracing-subscriber

Support:

--verbose
--debug
--log-file <path>

Create rotating logs.

Never log sensitive pairing material or Apple credentials.

======================================================================
PERFORMANCE STATISTICS
======================================================================

Display an optional live statistics overlay:

Device:
iPhone model
iOS version

Video:
transport
codec
source resolution
source FPS
decoded FPS
presented FPS

Performance:
dropped frames
receive latency
decode time
render time
frame queue depth
CPU
GPU where practical
memory

Input:
BLE / WDA
input request latency

Distinguish:

SOURCE FPS

from

DISPLAY FPS.

======================================================================
BENCHMARKS
======================================================================

Implement benchmark instrumentation.

Measure:

- average FPS
- 1% low FPS where meaningful
- frame-time variance
- dropped frame percentage
- decode latency p50/p95/p99
- render latency p50/p95/p99
- memory usage
- CPU usage
- reconnect time

Output benchmark JSON.

Example:

benchmarks/
  iphone15-usb.json
  iphone15-airplay.json

======================================================================
AUTOMATIC QUALITY SELECTION
======================================================================

Implement:

Auto
Quality
Balanced
Low Latency

Auto should select based on actual hardware.

Low Latency:

- minimum buffering
- latest-frame rendering
- hardware decoder
- 1080p target
- 60 FPS request

Quality:

- maximum useful source resolution
- higher quality where transport supports it

Never upscale a low-resolution source and call it "native".

======================================================================
DEVICE CAPABILITY DETECTION
======================================================================

On startup detect:

- Windows build
- CPU
- GPU
- hardware H264 decoder
- hardware H265 decoder
- Bluetooth adapter
- BLE peripheral capability
- network interfaces
- iPhone connectivity
- Apple device driver availability

Store diagnostics.

Create:

Help -> Diagnostics

with "Copy diagnostics" button.

======================================================================
DEPENDENCY PACKAGING
======================================================================

The installed application must NOT require the user to manually install:

- Rust
- Cargo
- Git
- Node
- Python
- CMake
- Ninja
- FFmpeg CLI
- developer SDKs

Bundle or bootstrap legitimate redistributable runtime dependencies when licensing allows it.

Where Apple-provided drivers are legally/technically required:

detect them and give the user an explicit supported installation path.

Do not illegally redistribute Apple binaries.

======================================================================
WINDOWS RUNTIME
======================================================================

Prefer static Rust CRT where appropriate:

target-feature=+crt-static

only if compatible with all native dependencies.

Otherwise bundle/install required Microsoft runtime correctly.

The final EXE must not depend on random DLLs existing in the developer's PATH.

Run dependency inspection on the final binary.

======================================================================
BUILD PROFILE
======================================================================

Create optimized Cargo release settings.

Evaluate:

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"

Do not blindly optimize for binary size if doing so reduces runtime performance.

Benchmark:

thin LTO
fat LTO

if useful.

======================================================================
BUILD COMMANDS
======================================================================

The repository must support:

cargo build
cargo build --release
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

Create:

scripts/setup.ps1
scripts/dev.ps1
scripts/test.ps1
scripts/build-release.ps1
scripts/package.ps1
scripts/diagnostics.ps1
scripts/clean-machine-test.ps1

PowerShell scripts must use:

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

======================================================================
CI
======================================================================

Create GitHub Actions.

Required jobs:

format
clippy
unit-test
windows-build
release-build
installer-build

Build release artifacts for:

windows-x64

Later:

windows-arm64

Cache Cargo dependencies safely.

======================================================================
RELEASE ARTIFACTS
======================================================================

A release must produce:

dist/
  iMirror.exe
  iMirror-Setup-x64.exe
  iMirror-x64.msi
  checksums.txt

Optional:

portable/
  iMirror.exe
  required DLLs

Users must be able to:

download installer
→ double-click
→ install
→ launch

without opening Terminal.

======================================================================
INSTALLER
======================================================================

Prefer WiX Toolset for MSI.

Optionally create a bootstrapper EXE.

Installer requirements:

- per-user install where possible
- Start Menu shortcut
- optional Desktop shortcut
- uninstall entry
- clean uninstall
- app data preserved only where appropriate
- firewall rule requested only when actually necessary
- no unexplained administrator requirement

Support:

silent install
silent uninstall

for future enterprise usage.

======================================================================
FIRST-RUN EXPERIENCE
======================================================================

First launch:

1. Scan for iPhone.
2. Show device.
3. Validate required drivers.
4. Explain "Trust This Computer" if needed.
5. Check video backend capability.
6. Check Bluetooth HID capability.
7. Recommend best mode.
8. Connect.

Do not expose implementation complexity unnecessarily.

Advanced users can open Settings.

======================================================================
RECOVERY
======================================================================

The application must survive:

- unplugging USB
- plugging USB back in
- iPhone locking
- iPhone rotating
- Wi-Fi loss
- AirPlay interruption
- Bluetooth disconnect
- Bluetooth reconnect
- sleep/wake
- monitor change
- DPI change
- renderer device loss

Recover automatically where technically possible.

Never crash because a phone was unplugged.

======================================================================
MEMORY SAFETY
======================================================================

Rust unsafe code is permitted only where required for:

- Win32
- D3D
- native FFI

Every unsafe block must have a:

// SAFETY:

comment describing its invariants.

Minimize unsafe surface area.

======================================================================
OPEN-SOURCE FFI
======================================================================

Do NOT waste months rewriting proven C/C++ protocol implementations merely to achieve "100% Rust."

Allowed architecture:

Rust application
      ↓ FFI/process boundary
mature native OSS engine

But:

- isolate it behind a Rust abstraction
- pin upstream version
- document patches
- build reproducibly
- enforce license obligations

If helper processes are used:

- ship them with installer
- manage lifecycle
- hide console windows
- collect logs
- kill cleanly on exit

======================================================================
LICENSE SAFETY
======================================================================

Perform a complete license audit.

Generate:

THIRD_PARTY_LICENSES.md

and appropriate license files in installer.

Pay special attention to GPL/LGPL dependencies.

If UxPlay GPL code creates obligations for distributed binaries:

explain them before final architecture is frozen.

Do not incorrectly claim that a process boundary automatically avoids GPL obligations.

The project should remain open source unless licensing architecture deliberately supports another model.

======================================================================
SECURITY
======================================================================

No remote network access by default.

Bind local control services to:

127.0.0.1

where applicable.

Do not expose WDA publicly.

Validate all device-originated data.

No command injection through subprocess arguments.

No secrets in logs.

======================================================================
TESTING
======================================================================

Create:

unit tests
integration tests
stress tests

Important tests:

coordinate mapping
rotation
DPI
packet parser
frame queue
reconnect state machine
error recovery
config migration

Run long-running tests where hardware permits.

======================================================================
CLEAN WINDOWS VALIDATION
======================================================================

A release is NOT considered complete merely because it works on the development machine.

Validate installation on a clean Windows environment.

Use:

Windows Sandbox
or
clean Windows VM

The clean system must NOT contain:

Rust
Visual Studio
Git
Python
Node
FFmpeg development install

Test:

install
launch
uninstall
reinstall

Identify every missing runtime dependency.

Fix packaging.

Repeat until clean-machine installation succeeds.

======================================================================
REAL HARDWARE VALIDATION
======================================================================

Do not fake iPhone integration tests.

When physical hardware is required:

pause only that test and tell me exactly what action is needed.

Example:

"Connect your iPhone by USB and press Trust."

Then continue immediately.

Record:

- iPhone model
- iOS version
- transport
- source resolution
- source FPS
- CPU
- GPU
- Bluetooth chipset
- measured results

======================================================================
PERFORMANCE RELEASE GATES
======================================================================

Do not block release solely because the source cannot provide 60 FPS.

Instead require:

IF source is 60 FPS capable:

- renderer must sustain source FPS
- frame drops < 1% under normal operation where hardware permits
- queue must remain bounded
- latency must not progressively increase

IF source is 30 FPS:

- present exactly what is received
- do not frame-duplicate and claim 60 FPS

For USB on supported modern hardware:

TARGET:

source: 60 FPS where available
render: source FPS
decode/render processing p95: < 20 ms
memory: stable
no unbounded growth
CPU: reasonable
GPU decoding active when supported

For wireless AirPlay:

optimize aggressively but document actual measured latency.

======================================================================
STRESS TEST
======================================================================

Run at least:

- 30 minute continuous mirror
- repeated rotation
- 50 disconnect/reconnect cycles where automated
- resize spam
- fullscreen enter/exit
- device lock/unlock
- Bluetooth reconnect

Check:

- memory leaks
- deadlocks
- panics
- latency accumulation
- handle leaks

======================================================================
DEFINITION OF DONE
======================================================================

The project is NOT done until all achievable items below are true:

[ ] cargo fmt passes
[ ] clippy passes with warnings denied
[ ] tests pass
[ ] release build succeeds
[ ] Windows x64 EXE produced
[ ] MSI produced
[ ] Setup EXE produced
[ ] clean Windows installation tested
[ ] application launches without developer tools
[ ] real iPhone detected
[ ] real iPhone screen rendered
[ ] real tap works
[ ] real drag/swipe works
[ ] keyboard works
[ ] reconnect works
[ ] orientation change works
[ ] FPS is measured correctly
[ ] frame queue is bounded
[ ] memory remains stable
[ ] hardware decoder is used where supported
[ ] installer uninstalls cleanly
[ ] third-party licenses included
[ ] README contains end-user installation instructions

Anything requiring unavailable physical hardware must be marked:

BLOCKED: HARDWARE VALIDATION REQUIRED

not "completed."

======================================================================
WORKING METHOD
======================================================================

You have permission to make reasonable engineering decisions.

Do not repeatedly ask me which dependency to choose.

Research.
Choose.
Implement.
Compile.
Run.
Inspect errors.
Fix them.
Compile again.
Test.
Package.
Validate.

Do not stop after creating scaffolding.

Do not stop after writing TODOs.

Do not return a code dump and tell me to finish it.

Do not replace difficult native work with mocks.

Mocks are allowed only in automated tests.

When compilation fails:

read the actual compiler output and fix it.

When an open-source dependency does not build:

inspect its upstream build process
and patch/integrate it reproducibly.

When architecture assumptions prove wrong:

change architecture instead of forcing a broken solution.

Maintain:

docs/ENGINEERING_LOG.md

recording major decisions, blockers, benchmarks, upstream patches,
and commands required to reproduce the build.

======================================================================
INITIAL EXECUTION
======================================================================

Start now.

PHASE 0

1. Inspect the host Windows development environment.
2. Verify Rust MSVC toolchain.
3. Audit candidate upstream repositories.
4. Audit licenses.
5. Choose USB video architecture.
6. Choose AirPlay architecture.
7. Choose input architecture.
8. Write docs/ARCHITECTURE.md.
9. Create Cargo workspace.

Then immediately continue to implementation.

Do not wait for approval unless:

- an irreversible system operation is required,
- Apple/iPhone physical interaction is required,
- signing credentials are required,
- license constraints require a product-level decision.

======================================================================
FINAL OUTPUT
======================================================================

At successful completion I expect:

Repository builds from a fresh clone.

Running:

powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1

must produce installable artifacts.

Final response must include:

- architecture actually used
- upstream projects actually reused
- licenses
- exact build command
- EXE path
- MSI path
- Setup EXE path
- clean Windows test result
- real-device test result
- actual measured FPS
- actual measured resolution
- actual measured latency
- known limitations

Never report a test as passed unless it actually ran.

Begin.