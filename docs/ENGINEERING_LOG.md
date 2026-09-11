# Engineering log

## 2026-09-11 — Initial audit

The workspace was empty. Rust 1.96.0 MSVC x64 and VS 2022 Build Tools
17.14.33 are installed. Windows 11 IoT Enterprise LTSC build 26100;
Ryzen 5 6600HS; GTX 1650 plus AMD integrated graphics; Realtek Bluetooth.
Apple Mobile Device Service is running; Bonjour is installed but stopped.
No iPhone is attached. WindowsSandbox.exe is absent. No hardware tests have run.

Source repositories were cloned into ignored `research/` folders for inspection.
Source SHA records, licenses and selected subsets are persisted under vendor.
The host's restricted process launcher intermittently fails before execution;
read-only/source-build commands have required a sandbox escalation.

QVH's README explicitly abandons Windows support. ioscpy requires a jailbreak.
go-ios now has CoreDevice encoded display streaming but its source documents
iOS 27+ only; this cannot cover the iOS 17+ requirement.
RayrenSX/iPhoneMirror has a native QuickTime/MF/D3D11/WASAPI core with a C ABI,
shared GPU textures, a bounded video worker and protocol regression fixtures.
Reuse only its GPL core and dependencies, with our Rust app and controllers.
Do not import its non-OSI iUsbBridge or its WPF application.

GPLv3 implications were explained before selection: iMirror remains GPLv3,
with corresponding source and notices. Process isolation is not a GPL exemption.

Validation status: all real-device checks BLOCKED: HARDWARE VALIDATION REQUIRED.
Clean-machine installation still requires a clean Windows VM/Sandbox environment.

## 2026-09-11 — Native integration, packaging and first real phone

Continued the incomplete AirPlay integration. Fixed GUID formatting, staged all
required GStreamer plugins, and checked actual graceful receiver shutdown.
Added version-specific private plugin paths and replaced inactivity-triggered
wireless resets with a native disconnect event.

H.264 regressions now cover fragmented parameter sets, fragment-header mismatch,
whole-access-unit discard after packet loss, timestamp deduplication, unrelated
RTP payload types and bounded memory. No screenshot stream is used for video.

Added persisted native Settings, receiver name, quality profiles, VSync and
Fit/1:1 with matching pointer geometry. Exposed actual decoder mode and successful
unique render submissions. Settings use atomic replacement and a valid backup;
future versions are not silently overwritten.

The first real USB checks exposed a missing configuration helper. Cargo now
builds UsbConfigurationSwitch from the pinned native source for both development
and release, and the installer includes it. After this correction a real
iPhone15,4 on iOS 27.0 supplied 1180x2556 video. A short headless run measured
59.6 source FPS; the user subsequently confirmed smooth native picture and
correct orientation. This host's Win32 driver inventory did not list libusb or
UsbDk services; no driver installation was performed by this continuation.

Visible runs exposed two measurement/pacing issues. Protocol reconnects reset
the USB source counter and could underflow FPS subtraction; the capture session
now accumulates an offset across resets. Shared GPU frames are released after
sampling work is submitted and before nonblocking Present; a busy compositor
retries the newest frame. Active media sessions balance timeBeginPeriod(1) /
timeEndPeriod(1) so native 1 ms waits do not rely on the default timer period.
The last short timer run received 1,251 frames and submitted 1,249 in 30 seconds.
The controlled long validation follows after Auto-Lock was disabled by the user.

References for these implementation choices:
- https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/dxgi-present
- https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timebeginperiod
- https://gstreamer.freedesktop.org/documentation/gstreamer/running.html
- https://www.rfc-editor.org/rfc/rfc6184

Built a 34 MiB-class AirPlay runtime, hash-pinned MSYS build inputs, source
download/build scripts, license inventory, WiX MSI/Setup and portable/source
ZIPs. Rust/C++ runtimes are statically linked where applicable; end users do
not install developer tools. Dynamic LGPL runtime DLLs stay replaceable.
WiX's bootstrapper source/notices are included under MS-RL. Native source
archives and every Cargo.lock dependency accompany the source package.

The first development-host MSI lifecycle passed installation, hash verification,
PATH-isolated startup, graceful AirPlay shutdown and uninstall. That early
package predates the final helper/pacing corrections and must be rebuilt and
retested. WiX component GUIDs are deterministic per file; per-user registry
keypaths and folder cleanup pass validation. ICE91 is specifically suppressed
because the package intentionally supports only per-user installation; other
MSI warnings are treated as errors.

Build entry point:
powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1

Validation limitations and uncompleted gates remain in VALIDATION.md.


## 2026-09-11 - BLE advertising and first-input milestone (incomplete)

Scope limited to BLE advertising, observed service/subscription state, a modeless
BLE diagnostics window and four explicit report probes. No CaptureSession,
transport, decoder, renderer, audio, FPS, watchdog or reconnect sources changed;
compared their hashes against work/ble-milestone-baseline.json.

The app now observes AdvertisementStatusChanged plus the current status property,
records STARTED separately from the start request, and displays a 10-second
startup timeout. Pairing guidance requires actual STARTED. Mouse report readiness
requires an explicitly selected current mouse subscriber. Keyboard readiness also
requires that target's keyboard subscription. Move Right/Left emit one relative
X report (+40/-40); Left Click and Type A emit down/up pairs. These have NOT been
validated against an iPhone. No test report was sent during this milestone.

Local BLE state snapshots include adapter identity, peripheral capability, radio
state, service creation, advertising state/error, both subscription lists, selected
target and diagnostic result. No pairing keys are stored. Launch example:

    target/ble-milestone/release/iMirror.exe --ble-control --ble-status-file D:/airplay-iphone/work/ble-live-status-v2.json

A release was built in target/ble-milestone to preserve the running USB mirror
in target/release. The first app instance was left running after it began USB
capture. The latest BLE diagnostic instance reported radio_state=OFF,
peripheral=true, low_energy=true, hid_service_created=false, advertising_status=
NOT_REQUESTED and RadioNotAvailable (1). Both subscriber lists were empty.
STARTED is NOT confirmed. Windows Bluetooth must be turned on and BLE control
started again before requesting phone pairing. The 30-minute USB test remains
paused at the user's request; it is outside this milestone.

The two BLE readiness/report tests passed before adding radio-state details;
checks were rerun after that addition. Scoped Clippy uses a command-line exception
for the existing benchmark manual_is_multiple_of lint; benchmark.rs is unchanged.
The Computer Use kernel failed initialization (kernel assets path, os error 3),
including after reset; the orca executable is unavailable. Runtime observations
came from the actual app's local diagnostics, not an automated UI screenshot.

Physical pointer movement: BLOCKED: HARDWARE VALIDATION REQUIRED.


## 2026-09-11 - HID enumeration/subscription correction

User confirmed the previous build reached advertising STARTED and iPhone Bluetooth
Connected, with both HID subscriber counts still zero. This is recorded as a
failed HID-subscription attempt, not a successful control connection.

Compared the pinned windows-ble-hid implementation characteristic-by-characteristic.
Details and reference links are in BLE_HID_COMPARISON.md. Added Protocol Mode,
observable encrypted HID reads, matching protection levels/order, Battery
Read|Notify, SubscribedClientsChanged for both reports, session status callbacks,
and connected LE/Classic peer watchers. GAP Appearance cannot be set through the
public Windows API; the reference documents the same limitation. CCCDs are
Windows-generated; none were fabricated manually. Report Map remains exactly
113 bytes with the same SHA-256 as the reference.

The sole actual mouse subscriber is selected automatically; no movement test is
sent automatically. New UI diagnostics expose read counts, protocol/control
writes, subscription callback counts and a bounded event timeline. Connected LE
and Classic peers are separate from HID subscribers. Subscription callbacks and
actual SubscribedClients are authoritative; compilation and tests cannot satisfy
the hardware gate.

cargo build --release passed. Four BLE tests and formatting passed. App Clippy
passed with only the pre-existing benchmark manual_is_multiple_of lint excluded
on the command line; benchmark.rs was not changed. Protected media, USB, native
vendor, scripts and installer files match the milestone baseline hashes.

Launched target/release/iMirror.exe (PID 10896) with --ble-control and diagnostics
at work/hid-subscription-live.json. This fresh attempt observed radio OFF and
RadioNotAvailable before HID creation; it does not contradict the earlier user
confirmation of STARTED on the previous attempt. Asked the user to turn Bluetooth
on and start BLE control, then test a fresh iPhone pairing against the changed
GATT table. Hardware subscription acceptance remains pending. No test report sent.

After the user enabled Bluetooth and started BLE control, live PID 10896 reported
radio ON, HID service created and AdvertisementStatusChanged STARTED (error 0).
Protocol Mode creation succeeded. One LE connection was observed with an empty
name and paired=false in the watcher snapshot; identity is not confirmed.
HID read counts, GATT sessions and both subscription event counts remain zero.
Snapshot preserved as work/hid-started-before-repair.json. Fresh iPhone pairing
is the next hardware step; HID subscription has NOT passed.


## 2026-09-11 - Actual HID subscription, link loss, and USB recovery

The fresh iPhone pairing produced real ReadRequested callbacks for HID Information
and Report Map, followed by report ID 1 count=1 and report ID 2 count=1. The app
selected the real mouse subscriber. This passes the HID subscription milestone;
physical pointer movement was not tested at that point. The subsequent GATT
session closed and reopened, then closed again. Windows retained SubscribedClients
entries even while SessionStatus was Closed. The old UI could therefore incorrectly
indicate readiness after link loss. Original evidence is preserved in
work/hid-subscribed-then-disconnected.json.

Fixed BLE readiness and report dispatch to require a currently Active GATT session.
The UI separately exposes Windows stored/cached subscriptions and active subscribers.
An explicitly requested --ble-move-right-once diagnostic schedules a single +40 X
report after an active selected mouse subscriber remains ready for one second.
It marks the attempt before dispatch and never automatically replays an ambiguous
failure. Normal launches do not arm it. This is not reconnect polishing or new
keyboard mapping. Four BLE tests and scoped Clippy passed; release built.

User reported USB mirror unavailable. Initially iMirror discovery returned zero;
Windows still saw the phone and reported Code 10 for WPD/Ethernet child interfaces.
Apple Mobile Device Service was running and listening on 127.0.0.1:27015. A later
read-only ListDevices request returned one USB device, and a repeated iMirror query
then returned the trusted iPhone15,4/iOS 27.0. No service restart, driver change,
or mirroring source edit was performed. Opened the current app and invoked its
existing USB Connect command on the verified main window and single selected phone.
UI then reported streaming 1180x2556 with hardware decode; native logs confirmed
ongoing decoded frames and D3D presentation. No 30-minute test was run.

Windows System/BTHUSB events included a successful LE pairing (ID 8), older SMP
pairing timeouts/rejections (29/37), and Classic mutual-authentication failures (16)
for the iPhone public address. These are evidence, not proof of the complete cause
of LE disconnects. Asked the user to remove stale pairings on BOTH Windows and
phone and pair again through AssistiveTouch. Current PID 4660 remains open with USB
mirroring and the one-shot HID test armed; new live state is
work/hid-recovery-live.json. Stable Bluetooth control and actual pointer movement
remain unconfirmed.

Follow-up at 21:04:45 local time: PID 4660 was responsive with one Active
mouse subscriber and one Active keyboard subscriber. Last Active GATT transition
was at 21:01:21, with no later disconnect event in the retained timeline (about
3 minutes 24 seconds observed). The diagnostic result recorded Windows acceptance
of Move Right earlier and Move Left most recently. These are API/connection
observations, not physical cursor confirmation or a long-run reliability pass.
Snapshot: work/hid-active-after-cleanup.json. User visual confirmation pending.


## 2026-09-11 - Physical pointer confirmation and sensitivity adjustment

The user explicitly confirmed that the real iPhone pointer moves, then reported
that it was too sensitive compared with the Windows pointer. Physical BLE relative
pointer movement is now PASS for this phone; this does not validate taps, drags,
keyboard typing, all hardware, or long-run reconnect reliability.

Added PointerScale in input-core: default 25%, adjustable 5-200%, integer residuals
preserve sub-count movement, reversal cancels residuals, capture/target/gain changes
reset them, and overflow is discarded rather than queued. Scaling applies only to
physical BLE mouse motion; click transitions and wheel values are preserved, WDA
and keyboard mapping are unchanged, and fixed diagnostic steps stay fixed. Zero
motion duplicates are skipped while retaining button transitions. No extra
acceleration or video/viewport scaling was introduced.

The BLE control window has a native Mouse speed trackbar. Changes apply on the
input worker immediately and are saved after a short debounce to
%LOCALAPPDATA%/iMirror/ble-input.json, independently of video settings. Writes use
atomic replacement. Malformed/future preferences are preserved with an explicit
notice. Six input-core tests (four new scaling cases) and three app tests (two new
preference cases) passed. Scoped Clippy passed with only the existing benchmark
manual_is_multiple_of lint excluded; cargo fmt check passed.

Built the release in target/ble-milestone while the old app kept running. Then
closed the old app gracefully, updated target/release/iMirror.exe and launched PID
21904 without the one-shot movement flag. Restored the previous window geometry
and used the existing USB Connect action only if not already streaming. Native UI
verification confirmed range 5-200, applied 40% through the trackbar, restored 25%,
and verified saved JSON=25 and label Mouse speed: 25%. Both active HID subscriber
counts were 1; USB was streaming 1180x2556 with hardware decode. Slider changes did
not restart the receiver. The user's subjective calibration at 25% still requires
feedback. No media transport, GATT layout, decoder, renderer, capture/watchdog or
installer source changed (hash baseline: work/pointer-speed-baseline.json).
