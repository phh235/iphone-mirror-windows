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
was retained. Direct-touch software mapping and HID report tests passed; exact
physical click placement remains unconfirmed and must not be reported as passed.
