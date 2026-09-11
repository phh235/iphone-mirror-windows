# Validation status

Current milestone: native Windows 11 UI and low-latency Bluetooth mouse control.
This is an engineering build until the new UI/control/packaging gates below pass.

## Verified pre-change behavior

- The user confirms smooth real iPhone mirroring and working relative Bluetooth
  pointer control. Device identifier: iPhone15,4; iOS 27.0.
- Latest unchanged-release baseline (30.056 seconds): 1180x2556, source 56.5880 FPS,
  renderer submissions 56.4474 FPS, hardware Media Foundation decoding.
  1,611 source frames, 1,606 submitted frames. Compilation overlapped the run.
- Sampled decode timings are local processing measurements; they are not complete
  per-frame percentiles or physical end-to-end latency.
- Earlier installer lifecycle and PATH-isolated startup checks ran on this
  development PC. They do not establish clean-machine validation for the new build.
- Audio remains removed by user request.

## Current software evidence

- Rollback: tag rollback/pre-native-ui-raw-input-20260911, commit 78f713c.
- Production control retains the relative mouse/keyboard HID implementation.
  Failed experimental digitizer code and its research are preserved in the tag.
- Strict cargo clippy --all-targets --all-features -- -D warnings passed after
  removing the obsolete integration and correcting the benchmark lint without
  changing its sampling condition. No lint exception was used.
- Final checks must be rerun after the upcoming Raw Input/UI implementation.

## Outstanding gates

- New Raw Input latency/cadence measurements and seven physical pointer UX tests.
- New UI visual verification, themes/high contrast/DPI and Ctrl+Alt+Q release.
- Before/after mirror comparison using the frozen transport/decoder/renderer.
- Fresh EXE, MSI, Setup and portable package with supported VC runtime deployment.
- Clean Windows install/launch/uninstall/reinstall. No clean-machine result exists.
- Sustained 30-minute mirror/control test and repeated real reconnect tests.
- Wireless real-device validation and optional signed WDA runner validation.

No physical responsiveness, click, drag, wheel or keyboard result for the new
implementation may be marked PASS before the user confirms it. No current long
hardware test is running. Raw evidence is kept under ignored work/ and benchmarks/.

See NATIVE_UI_MILESTONE.md for the current plan and ENGINEERING_LOG.md for history.