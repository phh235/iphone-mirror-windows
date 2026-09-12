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


## 2026-09-11 - Repository cleanup and hover-effect clarification

User requested removal of excessive generated files before pushing to Git.
Measured workspace size before cleanup: 10.669 GiB. Preserved one
current portable build with runtime DLLs and third-party notices in dist/portable.
Removed 33 verified generated paths: Cargo outputs, MSYS/build caches, downloaded
corresponding-source copies, unmodified research clones and obsolete installers/
source ZIPs. Source archives/tool packages remain regenerable from pinned recipes.
Small raw hardware evidence and work-root diagnostics were retained locally.

After cleanup: 69.26 MiB total, including portable runtime and local
validation evidence. Git-visible files: 276, 4.578 MiB;
largest file 0.461 MiB. Verified all 276 pre-clean source
hashes unchanged. No commit or push was performed; no remote is configured.
.gitignore now excludes all research and generated benchmark data.

Added scripts/clean.ps1 (PowerShell 7), which validates absolute workspace targets,
refuses in-use/reparse targets, preserves modified research clones and requires
an existing portable release. The post-clean portable startup smoke passed with
PATH restricted to Windows system directories. Native CTest passed 5/5 before
caches were removed; no rebuild was run after cleanup.

Portable PID 22988 then streamed real USB video at 1180x2556 with hardware decode.
The Mute control was absent and current-process native logs contained zero WASAPI
entries while video output continued, verifying the final audio-only native guard.
Current app diagnostics showed RelativeMouse with one active mouse subscriber;
DirectTouch exact placement remains unconfirmed on the phone.

The user clarified that icons and Control Center controls themselves lift/scale
on hover, not a Hover Text enlargement window. This matches UIKit's pointer lift
effect, documented by Apple for app icons and Control Center. The earlier Hover
Text suggestion was incorrect. No custom hover scaling exists in iMirror. Direct
touch sends contacts only on press/drag/release; it sends no hover motion. iMirror
does not claim to have disabled the operating system's pointer animations globally.
Reference: https://developer.apple.com/design/human-interface-guidelines/pointing-devices

## 2026-09-11 - Direct touch audit and production UI quarantine

The user reported a failed physical DirectTouch test: enable the profile, click
the mirrored screen, and no action on the phone. Record FAIL, not a successful
touch test or a mapping-unit-test PASS. No successful absolute touch is recorded.

Traced preview WM_LBUTTONDOWN/UP through parent message routing, the touch_ready
gate, viewport Mapper, the bounded input worker, HidPeripheral::touch_contact,
six-byte finger reports and NotifyValueForSubscribedClientAsync. It is real BLE
HID over GATT (0x1812, report 0x2A4D, report ID 2), adapted from MIT WinBleTouch
d80d659af53188c45d3d2966f86ec35c0849287f. No USB/Wi-Fi/WDA/XCTest path is used for
DirectTouch. No Apple account, signing, Developer Mode, jailbreak or phone app is
requested by this path. Upstream's Zoom Full Screen/1x/Controller-off setup remains
an unvalidated compatibility assumption for this phone.

Current and saved touch-named diagnostics had returned to RelativeMouse. No
retained touch DOWN/UP result establishes how far the failed click reached. The
UI has a silent pre-mapping return when touch_ready is false, but the available
evidence does not prove this branch handled that click. Exact runtime stop point
is unknown. Subscription and Windows notification acceptance cannot prove iOS
touch interpretation.

Removed only the normal toolbar entry and DirectTouch command handler; adjusted
the two Fit button indices for that removal. Explicit --ble-direct-touch remains
a diagnostic opt-in; no Automatic control mode exists (Auto is video quality).
No new backend, mapping change, report change, mirroring change, or synthetic click
was made. Preserved the pre-existing engineering-log edits. No app restart or
portable-binary replacement is part of this audit; the running EXE is unchanged.

## 2026-09-11 - Focused one-click DirectTouch debug build

User approved tracing and compatibility corrections before another hardware test.
Fetched pinned WinBleTouch Program.cs into ignored work/direct-touch-debug. Compared
its Report Map independently with the fixture: 78 bytes, exact match. A hand-copied
fixture initially had an extra zero; the independent upstream comparison caught
it and the corrected fixture and all ten BLE tests pass. No physical result is
inferred from those tests.

DirectTouch now advertises the standalone report ID 1 finger collection and no
keyboard collection. Relative HID reports are unchanged. Matched reference HID
Information, initial report, properties/protection, reference descriptor, battery
read-only advertising and validated 6-byte notification completion. Kept selected
client targeting, bounded waits and explicit discovery/write validation; differences
are documented in DIRECT_TOUCH_DEBUG.md. Context7's API-reference skill was read,
but its tools are not exposed in this session; pinned source and installed
windows-rs 0.61.3 bindings supplied the API details.

Added a bounded shared trace independent of the command queue and subscriber
snapshots. Records preview receipt before parent dispatch, readiness failures,
coordinates, queue rejection/cancellation, actual bytes and GATT notifications.
Advanced Diagnostics refreshes during Bluetooth waits; UI does no file I/O.
Explicit --direct-touch-one-click allows only one DOWN attempt and no drag reports.
No synthetic click, physical test, mirror restart or new backend was performed.
Native capture, video, audio, decoder, renderer, AirPlay and coordinate-map library
sources remain unchanged; only input-side mapping error reporting was expanded.

Commands:
    cargo test -p imirror-input-ble --locked --target-dir target/direct-touch-debug
    cargo build --release --locked --target-dir target/direct-touch-debug

Physical iPhone setup and the single center click remain pending user interaction.

Final software verification: cargo fmt --check passed; all ten BLE tests passed;
app release/all-targets Clippy passed with only the pre-existing benchmark
clippy::manual_is_multiple_of lint excluded. The final cargo build --release
--locked --target-dir target/direct-touch-debug completed successfully (18.55s
incremental build). Protected media/vendor/benchmark paths have no diff.

New staged EXE: dist/direct-touch-debug/iMirror.exe, 3,013,120 bytes.
SHA-256: df4bc10e893980e34b59e6ff0858d06555cf866d18cc68fec904b2b26966ae7d.
Verified it matches the release output and differs from the previous portable EXE.
Runtime files and notices accompany it; Start-DirectTouch-Debug.cmd supplies the
explicit one-click flag. Build marker: direct-touch-one-click-20260911-01.
Read-only --version process exited 0 with no stderr. An initial PowerShell direct
GUI-subsystem invocation printed the version but did not set LASTEXITCODE, so the
exit was rechecked using Start-Process -Wait -PassThru. No advertising, synthetic
click, physical click, app mirror session or installer test ran in this pass.

The trace also writes a per-process timestamped archive to prevent a later process
overwriting the click evidence. Stage manifest and version-check evidence are in
dist/direct-touch-debug/build-manifest.json and work/direct-touch-debug respectively.

## 2026-09-11 - First traced physical DirectTouch click

After the user confirmed the Zoom setup ready, Windows Computer Use failed twice
with native pipe unavailable (os error 2), including after kernel reset. No
PowerShell UI injection was substituted. The user manually closed/reopened the
diagnostic launcher. An overlapping earlier RelativeMouse process coincided with
new-provider ABORTED; after the user reopened one instance, PID 22752 reached
DirectTouch STARTED, one active digitizer subscriber, a fresh Report Map response,
selected target and no readiness blockers. No pairing change was needed for that
observed fresh descriptor read/subscription. Initial USB discovery was empty, then
the user connected video; native logs confirmed 1180x2556 output from this PID.

The user clicked before the next instruction. The first recorded click occurred
at client physical (352,312), normalized (0.612663,0.306183), integer HID (6127,3062),
not the requested center. At DOWN, profile=DirectTouch, capture state=4, readiness
true both in UI and worker, selected subscriber active and Report Map read true.
Queue accepted DOWN. Actual bytes: 03 00 EF 17 F6 0B. GATT returned Success (0),
BytesSent=6/6, ProtocolError=None. The subsequent UP bytes were 02 00 EF 17 F6 0B,
also Success (0), 6/6, no protocol error. Both notification calls completed about
3 ms after their logged starts; this is local API time, not physical touch latency.
The log contains exactly one successful DOWN and one successful UP result for this
attempt. Later clicks were blocked and the user later selected RelativeMouse.
The bounded trace retained all first-click events despite later cancellation noise.

Evidence: work/direct-touch-debug/hardware-click-22752.json, copied from the
per-process archive in dist/direct-touch-debug. This proves Windows notification
completion, not iOS touch interpretation. No additional click, drag, keyboard test, source fix or mirror change was
performed in this hardware-observation pass.

The user then confirmed live video, selected a conflicting "The iPhone responded"
option, and immediately clarified in their own Vietnamese text: "tôi ấn nhưng ko
thấy phản hồi gì cả" (clicked but saw no response). The latest explicit written
clarification is authoritative: physical response for this attempt is FAIL.
Windows API completion remains observed; no iOS-side trace or radio packet capture
establishes where the report was ignored. This is not proof that every iOS device
rejects absolute HID. The actual point was not the requested center and its UI
target was not independently inspected. No second touch was sent. DirectTouch
remains experimental and excluded from normal/automatic control.

## 2026-09-11 - Native UI / low-latency control milestone begins

User authorized UI replacement, Raw Input control optimization and packaging,
while freezing the working media pipeline. Snapshot commit 78f713c and rollback
 tag rollback/pre-native-ui-raw-input-20260911 preserve the entire previous state.
Working branch: work/native-ui-raw-input. Frozen hashes cover 198 media files.
Direct Touch is removed from production source, command-line paths, state,
normal documentation and license inventory. Its implementation remains in the
rollback tag. Shared relative BLE service/report descriptors and encrypted
metadata are retained. Strict Clippy passes without any lint allow override;
the authorized benchmark change uses is_multiple_of(5), preserving the exact
sampling schedule.

Two baseline attempts saw no USB phone. After user reconnection, the unchanged
release captured/rendered for 30.056 seconds: 1180x2556, 56.5880 source FPS,
56.4474 submitted FPS, hardware decoder; 1,611 received and 1,606 submitted frames.
The run overlapped compilation and is labeled accordingly. Raw evidence is in
work/native-ui-milestone/mirror-before-connected.json and companion samples.

## 2026-09-12 - Product hardening, stopping at the Phase 5 hardware gate

Preserved the complete audited working tree in four logical commits (afd9e07,
758f624, 93bfeaa, 7549f09) and tag checkpoint/native-ui-ble-working. The tag's
Git archive built successfully with cargo build --release --locked --offline in
an isolated source and target directory; no untracked source was required.

Hardening adds subscription/session generation invalidation, fresh subscriber
verification and neutralization before readiness, immediate independent Raw
Input release, and a 128-transition cap with all-UP safety on saturation. The
one-slot movement accumulator and its scaling/pacing policy remain intact.
Both diagnostic and normal HID sends now share failure invalidation. Neutral
reports do not inflate physical input latency statistics.

Settings pairing guidance requires STARTED; WDA has separate readiness and Home
requires a ready capable backend. A first scrolling implementation passed
geometry checks but its small-window image exposed a blank end position. Fixed
that with a clipped native content viewport, pinned navigation/Done and focused
control reveal. Also bounded viewport background painting for PrintWindow; an
oversized erase rectangle had obscured pinned controls in the diagnostic image.
Smoke validation now explicitly verifies the footer after final scrolling.

Diagnostics use a dedicated file thread, create parent directories first and
rotate a 2 MiB JSONL file with three bounded archives. Advanced Copy Diagnostics
exports useful host/video/control state and recent errors without UDIDs,
Bluetooth addresses, phone names, input text or pairing material. Storage errors
are visible; obsolete automatic Move Right wording is removed.

49 Rust tests cover the existing parser/config/coordinate behavior and new
subscription epochs, queue saturation/releases, neutral-sample exclusion,
guidance, diagnostic rotation and export privacy. Strict Clippy, formatting and
release builds pass. Settings navigation passed at five app-controlled test DPI
values (96/120/144/168/192) on all four pages and at two small-window sizes.
These are software UI checks, not actual monitor-DPI or iPhone acceptance tests.
The final test executable is staged separately; BUILD_MANIFEST.json identifies
its exact source/binary. The first staging candidate is superseded by the r2
folder after the viewport correction. Neither candidate was physically tested.

Frozen media files match the 198-file baseline. USB/QuickTime, decoder, renderer,
frame timing, audio and Valeria/09:41 behavior are unchanged. No Phase 6 benchmark,
30-minute stability run, current UxPlay deployment, installer build or clean-VM
test is run before the user's explicit acceptance of the Phase 5 executable.
See HARDENING_PHASE5.md for acceptance scope. Hardware result remains PENDING.

## 2026-09-12 — Compact native chrome and official Fluent SVGs

Removed the bottom toolbar reservation and made the top actions compact native
icon buttons with native tooltips and a narrow-window overflow menu. Settings
uses compact sections, a fixed sidebar and independently scrolling content.
The native trackbar remains responsible for input; custom painting supplies a
single themed track/thumb. Native button capture ownership and keyboard focus
fixes from 7579dae are retained; no phone-input algorithm was changed.

Replaced hand-drawn icon primitives with eight unchanged Microsoft Fluent System
Icons Regular SVGs pinned at 9cf8af0f95a555918a60b8147a2f33a6a1248442. MIT notices
and source hashes are in assets/fluent and are included by release staging.
Direct2D's native SVG parser creates cached vector geometries. A cached software
DC render target paints the UI's foreground color; no video device or context is
used. No icon framework, font dependency, PNG or SVG file I/O on paint is added.

Formatting, strict Clippy, 52 tests and release compilation pass. The exact
staged EXE passed 50 dark/light UI/DPI smoke checks and two small-window checks.
Three icon microbenchmarks exercised 120 palette/DPI cases each. The measured
native SVG cost is about 35–39 ms parse/cache initialization, 0.285–0.312 ms warm
paint average and about 2 MiB additional private memory in matched idle app runs.
All 198 frozen media hashes still match; BLE/Raw Input/ControlManager files are
unchanged. Full measurements, EXE hash and limitations: UI_POLISH_VALIDATION.md.

The initial idle harness accidentally wrote a BOM using PowerShell 5 UTF8, which
the JSON parser rejected. Fixed only the isolated test-profile writer to emit
UTF-8 without BOM; user configuration was preserved. A second run was rejected
when USB capture appeared. Final measurements include two verified idle runs
each for original r2, compact pre-SVG and compact SVG builds. No 30-minute or
physical first-click acceptance is claimed. Installer/public release work stays
behind the user's physical validation gate.
