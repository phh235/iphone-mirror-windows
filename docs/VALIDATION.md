# Validation evidence and outstanding gates

This is an engineering record, not a production release certificate.

## Completed on this host

- Rust MSVC 1.96.0; Windows build 26100; Ryzen 5 6600HS; AMD Radeon and
  NVIDIA GTX 1650; Realtek Bluetooth peripheral capability reported.
- Rust regression suite: 25 tests passed before the final timing changes.
  Final fmt, strict Clippy and all-target tests must be rerun before packaging.
- Native CTest suites: 5/5 passed, including real encoded-fixture decode,
  protocol bounds, source counters, output recovery and USB restore policy.
- Optimized x64 EXE, per-user MSI, Setup EXE and portable ZIP produced.
- Initial MSI installed successfully on the development PC; all 323 installed
  files matched the generated manifest. Native-window and AirPlay lifecycle
  smoke checks passed with PATH restricted to Windows system directories.
  Silent uninstall exited 0 and removed the application directory.
- AirPlay helper rebuilt from pinned UxPlay and minimal AAC/ALAC source.
  Staged runtime startup and graceful shutdown passed. No wireless phone
  stream has yet been validated.
- Corresponding-source ZIP assembled; a clean extraction/build still needs
  verification. The package must be refreshed after final source/document edits.
- User confirmed actual USB picture, smooth motion and correct physical
  orientation changes in the native diagnostic preview.

## Real USB observations

Device reported model identifier iPhone15,4 and iOS 27.0. These observations do
not establish support for every iPhone or the entire iOS 17+ target range.

| Run | Evidence |
|---|---|
| Short headless capture | 179 source frames; 1180 x 2556; about 59.6 FPS after first observed frame; hardware decoder reported |
| 30-second visible preview | 1,230 source frames; 1,170 successful unique render submissions; about 42.9 source FPS; user confirmed picture and orientation |
| 45-second nonblocking preview | 1,804 source frames; 1,724 submissions; 41.31 source / 39.59 render FPS |
| 30-second timer-resolution preview | 1,251 source frames; 1,249 submissions; 46.36 source / 46.44 render FPS; small rate differences reflect sampling boundaries |
| Earlier long attempts | Interrupted/cancelled, or ended after about 246.7 seconds with no-video timeout; these did not pass the 30-minute gate |
| New 30-minute run | In progress after the user explicitly disabled Auto-Lock; final evidence pending |

Raw device reports live in ignored benchmarks/*.json and *.samples.jsonl.
Long runs stream samples to disk and retain only 300 in memory. Process CPU,
working-set, private-memory and handle samples are separate resource JSONL files.
Source FPS is based on distinct source timestamps, not different pixel content
or UI refresh count. Render counters count accepted unique Present submissions,
not proof of physical scan-out timing.

Decode values are actual individual local decode timings sampled by the
benchmark. They do not establish a per-frame p95 distribution or end-to-end
latency. End-to-end latency remains unmeasured.

## Outstanding release gates

- Complete 30-minute capture/render run and review resource stability.
- Real tap, drag, keyboard, sustained BLE reconnect and optional WDA actions:
  **BLOCKED: HARDWARE VALIDATION REQUIRED**; pairing and signed WDA setup
  require user/device interaction.
- Physical USB reconnect cycles, lock/unlock recovery, monitor/DPI transitions,
  fullscreen and resize stress still require end-to-end validation.
- Wireless real-video/control behavior and HEVC. Audio has been removed by user request.
- Full per-frame processing percentiles, physical latency, comprehensive live
  metrics and automatic benchmark-driven quality.
- Clean Windows install/launch/uninstall/reinstall: no clean VM or Windows
  Sandbox is available on this host. Desktop-host tests are not clean tests.
- Automated visual screenshots: Computer Use initialization failed with
  "failed to write kernel assets ... path ... (os error 3)" even after reset.
  User visual confirmation is recorded separately from automated checks.
- Release binaries are unsigned; no signing credentials were supplied.
- hex-slice 0.1.4 omits a standalone notice in its published crate. Its MIT
  declaration/authorship from Cargo.toml and standard license text are included,
  and the exception is explicit in the generated inventory.

An unchecked requirement in PRODUCT_REQUIREMENTS.md remains outstanding.


## BLE pointer confirmation and sensitivity update

On 2026-09-11 the user confirmed physical iPhone pointer movement through BLE HID.
Real HID Information/Report Map reads and mouse/keyboard subscriptions were observed
before this confirmation. This establishes relative pointer movement on the tested
phone, not complete mouse/keyboard UX or long-run reliability.

The sensitivity update runs with a 25% default and a live 5-200% native slider.
Automated native-control verification applied 40%, restored 25%, and checked the
saved preference; USB video and both active HID subscriptions remained available.
Subjective Windows/iPhone speed matching awaits user calibration; exact matching
is not guaranteed because the operating systems may apply different acceleration.


## Video-only and direct-touch update

Audio was removed from the application UI and receiver path. UxPlay video-only
startup/shutdown passed with actual `-as 0` arguments. USB's disabled-audio branch
was corrected to avoid starting a silent WASAPI worker; protocol/liveness handling
was retained. Direct-touch software mapping and HID report tests passed; those
checks do not validate physical touch delivery.

## Direct touch physical failure and UI quarantine (2026-09-11)

**Physical test: FAIL (user reported).** The user enabled Direct touch and clicked
the mirrored image; the physical iPhone did nothing. No successful absolute touch
has been validated in this project. The earlier real pointer movement PASS was
for RelativeMouse, not DirectTouch. This is implemented experimental BLE HID code,
not a placeholder, but it is not a working production touch feature.

Read-only inspection found the running PID 22988 snapshot in work/portable-live.json
had already returned to RelativeMouse, with zero touch subscribers and a subscriber
unavailable error. work/direct-touch-live.json also contained RelativeMouse. Neither
snapshot identifies the stop point of the reported DirectTouch click. No retained
Touch DOWN/UP or Absolute touch result was found in the local JSON/log evidence.
Diagnostics are overwritten on state changes, not a complete click trace.

At the time of that failed attempt, the UI silently rejected a DirectTouch press before mapping if touch_ready() was
false. This requires an active selected report subscriber, a fresh Report Map read,
observed advertising STARTED, report protocol mode and no suspension. Further gates
include the streaming viewport, the bounded input queue, fresh discovery and an
active GATT session at notification time. Even successful Windows notification
does not establish that iOS interpreted the report as touch. The exact stop point
of this physical attempt is UNKNOWN; do not attribute it to iOS rejection, mapping,
or a missing subscription without a trace from that attempt.

Removed the toolbar button and its normal command handler. There is no Automatic
control selector in the current application; Auto is video quality only. The
existing --ble-direct-touch flag remains an explicit experimental diagnostic
opt-in. No transport, mapping, mirroring, or input report implementation was changed.
The existing running/portable EXE is unchanged by this source-only quarantine.

## Focused DirectTouch tracing pass (2026-09-11)

The user approved continued hardware debugging. Added live trace for the Windows
DOWN/UP messages, readiness and exact blockers, selected subscriber / Report Map
response, coordinates, queue, actual payload bytes and GATT result. Standalone
Report ID 1 descriptor now matches pinned WinBleTouch byte-for-byte; notification
success requires both success status and 6/6 bytes. See DIRECT_TOUCH_DEBUG.md for
remaining documented differences and required iPhone setup.

Ten BLE tests passed, including descriptor/reference, readiness blockers, partial
notification rejection and bounded one-click trace. These are software checks.
At that build-stage checkpoint, advertising, notification delivery and physical
touch had not run. The subsequent hardware attempt is recorded below.

Release build and read-only EXE startup check passed for
`dist/direct-touch-debug/iMirror.exe` (build marker
`direct-touch-one-click-20260911-01`). Verified SHA-256 differs from the old portable
binary and matches the freshly built release output. Formatting passed; app
release/all-targets Clippy passed with the existing unrelated benchmark
`manual_is_multiple_of` lint exception. These checks do not establish BLE delivery.

## First traced DirectTouch notification (2026-09-11)

New diagnostic PID 22752 reached STARTED, fresh Report Map response and one selected
active digitizer subscriber. The user-generated DOWN at client (352,312) mapped to
HID (6127,3062). Both UI and worker readiness passed, and the input queue accepted
the click. DOWN `03 00 EF 17 F6 0B` and UP `02 00 EF 17 F6 0B` each returned Windows
GattCommunicationStatus Success, BytesSent 6/6, no protocol error. No automatic
report was sent; additional DOWN attempts were blocked by the probe.

This was not a center-screen click (normalized position about 61% across, 31%
down). Windows GATT completion is observed. **Physical iPhone response: FAIL** —
the user's latest explicit text says they clicked but saw no response, overriding
the conflicting suggested-answer selection. Live video was confirmed by the user.
Evidence is retained at work/direct-touch-debug/hardware-click-22752.json.
Successful absolute touch remains unvalidated. No iOS-side event trace or radio
capture is available; the exact point of failure beyond Windows notification
completion is unknown. The clicked UI target was not independently inspected.
This result does not establish universal iOS incompatibility. No second test was
sent; DirectTouch remains experimental and excluded from normal/automatic control.
