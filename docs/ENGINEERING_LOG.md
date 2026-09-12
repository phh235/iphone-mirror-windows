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

## 2026-09-12 — User-supplied app logo

Used assets/logo.png unchanged as the application artwork. Generated a 12-size
ICO (16–256 px, 21,482 bytes) with the build-only prepare-app-icon.ps1 script and
embedded it as resource 101 using the Windows SDK resource compiler. SDK lookup
uses the host architecture rather than the full Rust target triple, as required
by find-msvc-tools 0.1.12 (already present through cc; no new runtime dependency).

Main/Settings windows load cached DPI-appropriate small/large native icons;
Advanced Diagnostics receives the same app identity. Only the diagnostic window
branding call changed in ble_panel.rs; no BLE/control code changed. WiX shortcut,
ARP and setup icon configuration now references the same ICO. No installer was
built or clean-machine-tested in this branding pass.

Formatting, strict Clippy, 52 tests and release build pass. Native main and
Settings smoke captures show the supplied logo in their title bars. Frozen media
and input implementations are untouched. The exact logo test build is staged at
dist/ui-logo-20260912/iMirror.exe, SHA-256
96a847662b530c2fe799a8db57a25f8838e95a0e4e5fd76bc93464f2c5cafa51.

## 2026-09-12 — Generated-output cleanup

Measured about 10.57 GiB in the workspace, primarily target (6.54 GiB), the
rebuildable MSYS/native dependency cache (3.06 GiB), uncompressed UI BMP captures
and duplicate staged runtimes. No files in target/work/dist were tracked by Git.

Updated scripts/clean.ps1 to select and hash-check the newest staged release,
support a read-only Preview, reject unsafe/in-use/reparse targets, and retain
source, vendor, research, Git history and rollback refs. Before deleting bulky
test evidence it archives and individually SHA-256 verifies the original files.
Small JSON/log/script evidence remains directly accessible. It does not repeat
archival when no bulky evidence files need removal.

Executed cleanup: removed 17 generated directories and 168 bulky files. Preserved
836 evidence files in work/evidence-before-cleanup-20260912-151731.zip with a
separate hash manifest. The workspace now measures approximately 83.22 MiB,
including the complete latest app folder dist/ui-logo-20260912 (41.85 MiB).
All 198 frozen media hashes and the retained EXE hash are unchanged. The retained
EXE passed an app-owned startup smoke with only Windows system directories on
PATH after target and the private development toolchain were removed. No phone
input or mirroring was started by this check. A second cleanup Preview reports
zero remaining cleanup targets. No Rust/media implementation was edited or
rebuilt during this maintenance operation.

## 2026-09-12 — Source cleanup, usage documentation and native UI follow-up

Removed the unused legacy ui.rs and moved the active production UI into that
canonical module without changing its implementation during the initial cleanup.
All eleven workspace crates have consumers; the decoder adapter is used by the
wireless path. Retained native components and runtime dependencies were not
removed merely because some optional paths are disabled. Historical audits and
the original requirements moved to docs/history with explicit snapshot labels.

Rewrote the Vietnamese and English usage guides around the actual compact UI,
removed stale audio/old-button/default-sensitivity instructions, and added build,
contribution, security-reporting and licensing guidance. Added a reproducible
source license index covering 105 external Cargo packages and pinned native/assets
notices. A local-link/UTF-8 checker validates first-party docs. CI definition now
checks format/docs, Clippy, tests, Windows release build and the license index;
this does not publish an installer or establish clean-machine validation.

The user then requested selectable Vietnamese/English UI and phone-proportional
normal windows. Added a small compiled UI catalog and persisted Language setting
under General. Native Window/Settings text changes immediately; technical
diagnostic keys/data and original driver errors are kept intact. Added window
geometry calculations using source ratio, measured text, monitor work area,
toolbar reflow, AdjustWindowRectExForDpi and WM_SIZING. Normal Fit uses about 88%
of work-area height with safe margins. Fullscreen/maximized and explicit 1:1/Fill
semantics remain separate. No video or BLE/Raw Input backend was modified.

Formatting, strict Clippy, 55 Rust tests and release build passed. The user tested
the exact new EXE (SHA-256 4b81337c06222cfca856d07198bcfa9e8096dd7f2a62f1a233061317e82602f2)
on the real iPhone: portrait has no unnecessary side bars; Rotate landscape fits;
fullscreen exit restores Fit; language switches immediately; manual resize stays
proportional; Disconnect/Connect refits. Actual viewport changed from 908x1228 to
564x1222 for the unchanged 1180x2556 hardware-decoded source at 144 DPI. Calculated
side padding changed from 170.5415 px per side to zero. These are actual HWND
measurements plus user confirmation, not a GPU pixel scan or FPS benchmark.
Physical multi-monitor/DPI and separate phone-orientation-change coverage remain
unconfirmed. See WINDOW_LAYOUT_VALIDATION.md for details and limits.

Validation was built in temporary work/source-cleanup-build to avoid leaving a
large target tree after maintenance. The prior runnable app remained intact
during development; raw evidence stays local and ignored by Git.

## 2026-09-12 — Compact startup, shutdown and USB recovery investigation

The old close handler exited the message pump before joining the control and
media workers, keeping a visible unresponsive window during native USB restore.
The new UI requests both workers to stop, releases local capture, hides main and
owned windows, and continues pumping until both worker groups finish. HWNDs stay
alive for the native preview until the final joins/destruction. The native USB
restore deadlines and protocol are unchanged. Stop checks prevent new queued
connect/discovery/control work starting after shutdown is requested.

The disconnected normal window now uses a short 440x230 effective-pixel client
area, bounded by the current work area and adjusted for DPI/non-client borders.
Existing source-driven Fit sizing takes over when video arrives. Explicit
disconnect returns to the compact panel after teardown completes. Fullscreen,
maximized and explicit display-mode behavior remains separate.

Formatting, strict Clippy, 56 tests and release compilation passed. Three visible
idle process tests hid windows in 4.24-7.11 ms and completed workers in
167.25-386.40 ms. Those intervals begin at WM_CLOSE, not a physical mouse click.

Hardware validation exposed a separate unresolved USB lifecycle issue: no PING
and libusb0 -116 reproduced on both candidate and previously verified reference.
All 80 compared runtime/helper files match. Restarting/unlocking/replugging the
iPhone recovered the reference, then the candidate mirrored 1180x2556 with the
hardware decoder and a 564x1222 viewport. The user confirmed live Fit and fast
close. That close hid the window in 5.58 ms but native USB restore timed out and
the process completed around 15.55 seconds later. Reopening initially found no
phone; cable replug recovered video automatically, confirmed by the user.

Do not count no-intervention reopen/reconnect as passed or call this a production
release. No driver installation, Apple-service restart or frozen native capture
change was made. See [validation details](STARTUP_SHUTDOWN_VALIDATION.md) for exact
EXE identity, observations, limits and local evidence paths. Detailed failure
history is preserved, including the initially optimistic user answer followed
by the cannot-connect report.

## 2026-09-12 — Windows WDA hardware setup and Wireless freeze diagnosis

Prepared Appium WDA v16.12.8 and go-ios v1.3.2 from checksum-verified release
assets. The user signed/installed WDA in Sideloadly using a personal Apple
Account, enabled Developer Mode and trusted the developer profile. The initial
Sideloadly Invalid file error later cleared; no specific fix for that signer
error was established.

Created a userspace RemoteXPC tunnel without installing a network driver. The
developer image 27A5228h was stored separately in local AppData, with payloads
checked against the pinned DeveloperDiskImage v0.3.0 repository. Apple TSS TLS
failed because Apple Root CA was absent from the Windows user trust store.
Downloaded the root from Apple's PKI site, verified its SHA-256
`b0b1730ecbc7ff4505142c49f1295e6eda6bcaed7e2c68c5be91b5a11001f024`, and imported
it into CurrentUser/Root with approval. TLS verification stayed enabled; image
personalization and mounting then succeeded. No system-wide trust store change.

The first real WDA launch failed with XCTest error 103. The device log explicitly
identified a missing code signature in the nested WebDriverAgentRunner.xctest.
A small native Go setup helper reused pinned go-ios transport to retrieve only
the provisioning profile matching this phone, bundle and Sideloadly certificate;
CMS signature and expiration checks passed. The profile expires September 19.
Used the existing local signing identity to re-sign the whole WDA package with
go-ios. The temporary P12 copy was removed; original Sideloadly files stayed
unchanged. Verified CodeDirectory CMS signatures for the app, XCTest bundle and
framework, then the phone accepted installation and loaded XCTest successfully.
Signing keys, profiles, signed IPA, pairing records and raw phone logs stay in
private AppData, outside this repository and release packaging.

WDA HTTP uses USE_IP=127.0.0.1 and USE_PORT=8100. For this local experiment the
optional screenshot broadcaster was assigned the occupied HTTP port; its log
confirmed EADDRINUSE and that it did not start. This workaround is not a general
production configuration. The setup helper forwards only 127.0.0.1:8100, with
at most eight connections, avoiding upstream go-ios forward's wildcard bind.

One real WDA tap at logical coordinates (58,656) selected Calculator key 1.
HTTP returned 200 in 726.13 ms; this is one software request duration, not an
end-to-end latency benchmark. Diagnostic screenshots showed 0 to 1 and the user
explicitly confirmed the physical phone changed without their touch. USB mirror
was off for this test. WDA standalone tap is verified; Rust UI click-to-WDA
integration is not yet physically verified.

Starting the existing QuickTime USB mirror removed the phone from USBMux and
terminated the WDA tunnel on this host, while USB video continued. No native
USB or driver modification was made. The user approved testing Wireless video
with USB WDA instead. The existing receiver connected and decoded an initial
498x1080 H.264 frame; WDA stayed ready and iMirror selected backend 2. The user
then reported the Wireless picture froze, so this is not a successful live-video
or combined-control acceptance test.

A separate 15.108-second RTP metadata capture used the unchanged staged UxPlay
and the exact production forwarding pipeline. It observed 662 packets, 496
markers with 496 different payload CRCs, zero sequence gaps, and exactly one
RTP timestamp. No video payload was persisted. Source review shows -vrtp skips
the code that sets the renderer's sync flag, and PTS is only assigned when that
flag is true. The Rust assembler correctly suppresses already-emitted timestamps,
explaining the single displayed frame. A proposed patch enables source PTS in
the RTP branch without changing sink synchronization or the Rust duplicate
guard. After the user's explicit approval, applied this narrowly scoped RTP
timestamp patch; USB, the Rust assembler, MF/D3D rendering, BLE and Raw Input
sources are unchanged. Local experiment source/evidence is under work/wda-setup.

Built the helper using scripts/build-airplay-helper.ps1 and the existing
SHA-256-pinned UCRT package lock. This helper-only build preserves the existing
runtime DLLs and does not rebuild FFmpeg/audio. The upstream -march=native
configuration remains a public binary portability limitation; this is a local
hardware-test candidate, not a portable public release.

Ran cargo fmt --check, Clippy with all targets/features and warnings denied,
cargo test (56 tests), and cargo build --release: all passed. The fresh Rust EXE
is 3,256,320 bytes, SHA-256
`afaf1234180d75a3ecbfed10d347fbb847ecf01e08ae99e76e17d466c24c3e65`.
The patched UxPlay is 704,214 bytes, SHA-256
`1221a519f639853a6aec494c8e40d1dd3dd344540c05285f13be75a3e91352dd`.
Staged together in dist/wireless-wda-20260912 with the retained runtime and
notices. Helper PE imports have no additions; application-local dependencies
exist. Windows successfully resolves the imported UCRT API-set contracts (these
are loader aliases, not missing physical DLLs). Other runtime files match the
retained candidate. Receiver startup/shutdown smoke exited successfully with
no phone stream; fixed timestamp behavior and combined Wireless/WDA physical
acceptance still require the next phone test. No installer or clean-machine
validation is implied.

### Follow-up: varying RTP timestamps, but no displayed image

The first patched physical probe received 682 packets, 596 markers, 596 distinct
timestamps and 596 distinct payload CRCs in 15.0009086 seconds; no sequence gaps
or malformed packets. This confirms that the constant-timestamp defect changed,
not successful decoding or display. The user then ran the staged EXE (PID 14356)
and reported no mirror image. Their diagnostics showed WDA ready, 498x1080 input,
764 submitted source frames, state 2 (WaitingForDevice), zero decode time and
software decoder status. The source counter increments before native decoding;
the software status alone is not proof of a completed software-decoded frame.
The native log showed decoder configuration but no decoded/rendered frame.

The RTP probe's first timestamp also jumped far from the next frame. Source
inspection and a regression test of the actual video_process callback reproduced
a second defect: on a pipeline-clock underflow retry it adds the entire remote
clock offset to an already-adjusted timestamp. The test delivered 20 seconds
then 16.64 milliseconds for two consecutive source frames. Saving the original
remote time before the retry makes those outputs 0 and 16.64 milliseconds;
the ordinary no-retry case also passes. The test uses a simulated renderer clock
and is explicitly not physical video acceptance.

Changed only the UxPlay callback for that retry correction. Rebuilt UxPlay and
reran fmt, Clippy with warnings denied, all 56 Rust tests and release build:
PASS. The new helper SHA-256 is
`ec6f5e4a3022970bb682f5a6e92d9a601f6bc25b7a8a6456deb3539179199c32`.
The Rust EXE is unchanged from the preceding rebuild. Staged the second candidate
in dist/wireless-wda-clock2-20260912 and started its existing AirPlay benchmark
with preview and separate native logs. It records source and render submissions
using the unchanged application path. This candidate still awaits phone video
acceptance; no decoder, renderer, USB, BLE or control-manager change was made.

The second candidate's 120-second AirPlay benchmark also failed: the user
reported an entirely black window. It submitted 1,521 encoded source frames but
zero render frames, ending with "Encoded frames arrived but no decoded frame
was confirmed". Native logs only show decoder configuration. Timestamp fixes
therefore do not establish that Wireless video works. Investigating the existing
three-slot EncodedSession queue with an instrumented local copy under
work/wireless-decode-probe; frozen production source is unchanged. The probe
counts overflow, waiting-for-keyframe drops, decode calls/results and generation
changes, using the existing H.264 assembler and real native decoder. It saves
metadata only and neither renders phone images nor sends WDA input.

The instrumented physical run has now isolated the black-screen mechanism:
807 assembled/submitted frames, one keyframe, zero parser errors/drops and zero
negative timestamp deltas. Decoder configuration ran once; its one decode call
successfully returned one decoded frame with no exception. Meanwhile the
three-slot encoded queue overflowed once, incrementing generation. The decoded
frame was discarded by the subsequent generation check; 803 later frames were
discarded while waiting for another keyframe. State remained WaitingForDevice.
Thus the decoder did work, but no decoded frame was published for rendering.
The queue probe has exited and stored counters only, with no phone image dump.

The needed follow-up is in the separate Wireless EncodedSession queue/startup
handling, not USB CaptureSession, the shared MF decoder or D3D renderer. This
extends beyond the specifically approved UxPlay timestamp patch and touches
previously frozen video buffering. No production queue change has been applied.
See WIRELESS_BLACK_SCREEN_DIAGNOSIS.md for evidence and the scoped next change.

The user subsequently approved the focused Wireless queue correction and asked
to proceed without repeated permission questions. Added EncodedFrameQueue:
startup at most 32 queued packets / 16 MiB payloads / 500 ms receive age, then
ordered drain and return to three packets / 250 ms age in steady streaming.
This absorbs the observed approximately 132 ms cold decoder configuration
without intentionally waiting or changing source timestamps. Startup allocations
are released as packets drain; steady streaming keeps reusable buffers. Genuine
overflow/expiry still requires a new keyframe but now sets an explicit Wireless
error. Generation validation/publication is serialized with stop/reset.

Only Wireless EncodedSession, the new queue header and focused native test/CMake
registration changed. USB CaptureSession, transport, MF decoder, D3D renderer,
BLE, Raw Input and WDA files remain unchanged. A deterministic queue test covers
20 packets arriving while consumption is stalled, FIFO preservation while
draining, count/byte/age limits, and 100 reset/wrap cycles. All six native CTest
groups pass, including encoded fixture decoding; fmt, warnings-denied Clippy,
56 Rust tests and release build also pass. Receiver start/stop and staged PE
dependency checks pass on this development host, not a clean Windows VM.

Opened the new candidate's existing AirPlay benchmark with preview. EXE:
dist/wireless-startup-fix-20260912/iMirror.exe, 3,259,392 bytes, SHA-256
`6c9d412596ec4a15126a436b023c0ea13f46bfb7f1b864fcd469a1004ba0c1d3`.
Helper SHA remains
`ec6f5e4a3022970bb682f5a6e92d9a601f6bc25b7a8a6456deb3539179199c32`.
Physical video and combined WDA click are pending; no success inferred from the
software tests. Details and limitations are in WIRELESS_BLACK_SCREEN_DIAGNOSIS.md.

The startup candidate rendered initially but the user reported it froze at the
connection image. Samples showed 15/27/39 received frames over consecutive
200 ms intervals; render submissions stopped at 18 and status entered queue
recovery (-3002). Final benchmark JSON had no top-level error despite the stall,
so that field is not used as a success gate. The original three-packet streaming
limit, restored after startup, is insufficient for this delivery/scheduling.

Retained startup bounds and changed only the compressed streaming bound to 16
packets, still constrained by 16 MiB queued payloads and 250 ms receive age.
Decoding remains immediate and the decoded latest-frame renderer is unchanged.
Added recovery-reason logging and once-per-second native decode/queue metrics
to verify the next run rather than guessing from a static image. Extended the
queue test with a bounded 16-packet streaming burst; all six native test groups
pass. The next exact release and real-device run are pending.

The burst candidate is now built and staged at
dist/wireless-burst-fix-20260912/iMirror.exe (3,260,416 bytes), SHA-256
`6c73b777548caeb5c1f97fee77d6c5431a8d050cab04a84ea1e4c0a179baa2cf`.
Fmt, warnings-denied Clippy, 56 Rust tests, release build and all six native
CTest groups passed. UxPlay remains the clock-corrected helper with SHA-256
`ec6f5e4a3022970bb682f5a6e92d9a601f6bc25b7a8a6456deb3539179199c32`.
Runtime checks and receiver startup/shutdown passed on this host. Started the
existing preview benchmark with fresh logs under local AppData
iMirror-WDA/wireless-burst-fix-20260912. Physical continuous video remains
pending; no WDA click was sent and no USB/BLE/decoder/renderer code changed.

The user confirmed continuous visible updates with the burst candidate. The
120-second scripted run (including idle phone intervals) recorded 1,285 encoded
source frames and continued decoded publication; the last periodic log counted
1,284 published frames. Peak encoded queue was four during startup and ten during
streaming, below the new 16-packet streaming bound but above the former limit of
three. No encoded_queue_recovery or MF runtime_failure occurred. Source was
498x1080. Whole-run source rate was 11.875/s including static intervals, not a
60 FPS full-motion validation. Render-submission rate was 11.280/s; internal D3D
logs also contain repeated submissions and implausible display-FPS readings,
which are not used as unique displayed/source FPS evidence. Last sampled decode
time was 2.518 ms. The native diagnostic label says software because it is cached
at configuration; actual hardware acceleration was not independently verified.
Short-run Wireless visible video is PASS by user confirmation, not long-run,
loss/reconnect, USB regression or clean-machine acceptance.

The old WDA runner exited at 22:07:48 with EOF/lost testmanagerd; exact phone-side
cause is unknown. Reused the existing USB tunnel, trusted signed runner and
loopback forwarder to restart it, without re-signing or sending input. WDA status
is ready again. Opened the exact burst candidate in its normal native UI; current
settings select Wireless and WDA, backend 2 reports ready and no error. Next is
the user's one Calculator key-1 click through the live mirrored UI. That
integration is still UNTESTED. Numeric evidence is retained in
work/wda-setup/wireless-video-acceptance.json; private runtime logs stay in AppData.

The user has now confirmed the integrated physical test: with the normal native
iMirror UI showing live Wireless video and WDA ready over USB, a Windows click
on Calculator key 1 changed the physical phone from 0 to 1. Record integrated
Wireless-video + WDA-tap as PASS for this exact burst candidate. No extra input
was sent after confirmation. Integrated tap latency was not measured; drag,
keyboard, unattended WDA recovery, long-term stability and clean-machine release
remain unvalidated. The native UI run continued publishing decoded frames with
no current queue recovery error. USB/BLE/Raw Input and shared decoder/renderer
source files remain unchanged by this Wireless fix.

## 2026-09-12 — WDA query latency, duplicate launch and Wireless Quality

The user later reported non-working clicks. Two iMirror processes were open; the
second failed Ctrl+Alt+Q registration and had no ready backend, while Control
was saved off. WDA itself still answered. Closing both and enabling the existing
WDA preference in one new instance restored clicking, which the user confirmed.
They then reported excessive delay. Five read-only geometry requests averaged
165.135 ms; every old tap unnecessarily fetched geometry before sending input.

Cached geometry per session, retaining explicit format/rotation refresh and
invalidation on request failure. Invalid-session recovery revalidates geometry
before a coordinate replay; a changed size rejects the stale point. Ambiguous
timeouts never retry input. Added bounded WDA timing diagnostics and cleared old
error text on successful connection. Added a per-user exclusive desktop-instance
lock before UI/control initialization; a real duplicate launch exited without
starting a second app. No media, BLE, Raw Input or coordinate-map source changed.

Fmt, strict all-target/all-feature Clippy, 63 Rust tests and release build passed.
Staged dist/wda-latency-20260912/iMirror.exe, SHA-256
`31b534857f3dd54b0a80ffe57940b7cd2e76128ba05d36158a1b50e9c0b8e0fb`.
The user confirmed functioning clicks and a slight subjective improvement.
54 observed HTTP taps had zero failures and only two setup geometry queries;
median software dispatch was 597.097 ms, mean 667.020 ms. This is not a paired
end-to-end before/after benchmark or proof of instant WDA control.

The user compared Wireless clarity/smoothness with prior USB + Bluetooth Mouse.
Changed only their saved existing Quality preset with a backup; no code change.
Actual source increased from 498x1080 to 664x1440, and the user confirmed sharper,
smooth video. No queue recovery/decoder runtime failure appeared. Requested
60 FPS remains unproven as sustained presentation FPS. Quality is retained for
this setup; the application's default remains unchanged.

The WDA runner exited with EOF/lost-testmanagerd during the sequence. Restarted
the runner; phone logs showed its server running, but the old forwarder could
not reach it. Restarting that forwarder restored /status. The user reconnected
WDA in Advanced and confirmed input with Quality video live. Exact failure causes
remain unknown; automatic helper lifecycle is not implemented. A later 11-tap
window had no errors and one geometry query, median 614.651 ms dispatch. See
WDA_PERFORMANCE_VALIDATION.md for evidence, limits and reproduction context.
