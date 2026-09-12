# Validation status

This file describes the current project, not a blanket compatibility guarantee.
iMirror remains an engineering preview. No current package has completed the
clean-Windows public-release gate.

## Evidence that exists

| Area | Evidence | Limit |
| --- | --- | --- |
| USB video | User confirmed live, smooth video and orientation on iPhone15,4 / iOS 27.0 | One setup; not every iPhone/driver/Windows version |
| Bluetooth relative control | User confirmed pairing, pointer movement and responsive movement/click/drag/wheel tests | Later source/UI changes need acceptance on their exact build |
| Emergency release | User's responsive-test reply included the requested Ctrl+Alt+Q sequence | Repeat for each release candidate and reconnect/focus-loss cases |
| Keyboard | Real implementation and software tests | Separate physical typing/layout coverage is incomplete |
| Native UI | Dark/light screenshots, app-owned five-DPI layout checks, small-window checks; supplied logo shown in main/Settings | Actual monitor changes, high contrast and 100-click physical stress remain pending |
| Phone-shaped window and language | User confirmed portrait without side bars, Rotate landscape, fullscreen restore, language switching, proportional manual resize and reconnect refitting | [Exact geometry and build hash](WINDOW_LAYOUT_VALIDATION.md); real multi-monitor changes remain pending |
| Software checks | Most recent pre-cleanup build passed fmt, strict Clippy, 52 Rust tests and release build | See engineering log for source revision and later check results |
| Packaged-folder launch | Latest retained app launched with system-only PATH after deleting build caches | Development PC, not a clean VM |

The historical 30.056-second USB run recorded 1180×2556, 56.588 source FPS,
56.447 renderer submissions/sec and hardware Media Foundation decoding. Compilation
overlapped that run. Two later diagnostics snapshots had identical source counters
while the user reported responsive video; they are not a comparable FPS benchmark.
Decode time and GATT completion time are software measurements, not end-to-end
phone latency. No guaranteed 60 FPS, resolution or latency is claimed.

## Remaining release gates

- Exact candidate on a real iPhone: USB, rotation/fullscreen, Bluetooth reconnect,
  pointer motion/stop, click, drag, wheel, typing, release and focus loss.
- Native UI first-click tests from inactive/captured/released/disconnected states,
  with no duplicate commands; real DPI/theme/high-contrast checks.
- Comparable 30-second mirror benchmark and sustained 30-minute USB/control
  tests with memory, handles, CPU, failures and reconnect measurements.
- Wireless discovery, connection, video, actual source FPS and reconnect on a
  real iPhone. Wireless 60 FPS has not been validated.
- Optional WDA runner deployment and control validation.
- Exact installer/runtime/source-license bundle: install, launch, USB/control,
  wireless where supported, reboot, uninstall and reinstall on clean Windows.
- Signing decision and a matching GPL corresponding-source archive before public
  binary distribution. Builds are unsigned unless a release states otherwise.

Audio is disabled. Failed absolute-touch experiments are excluded from production
and preserved at `rollback/pre-native-ui-raw-input-20260911`. Bluetooth Mouse is
relative pointer control; coordinate tests do not prove absolute phone touch.

## Evidence locations

[Engineering log](ENGINEERING_LOG.md) records decisions and actual commands.
[Historical audits](history/README.md) retain previous snapshots and binary hashes.
Machine-local raw evidence is ignored by Git under `work/` and `benchmarks/`;
cleanup can move bulky files into a hash-verified evidence ZIP. Historical paths
may therefore refer to archived files, not current downloadable releases.
