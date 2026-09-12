> Historical snapshot; see [current validation](../VALIDATION.md) and the [history index](README.md). This record is not current setup guidance.

# Native UI and relative mouse milestone

Rollback: commit `78f713c`, tag `rollback/pre-native-ui-raw-input-20260911`.
Working branch: `work/native-ui-raw-input`. Direct Touch research/code is preserved
in that tag and will not be compiled into the production application.

Frozen inputs: 198 files hashed in `work/native-ui-milestone/frozen-mirror-hashes.json`.
No native capture/transport/parser/decoder/renderer/audio, video queue, reconnect,
watchdog or frame timing changes. The existing benchmark's `% 5 == 0` Clippy
correction is explicitly authorized and must preserve the same measurement cadence.

## Baseline

Old release retained at `dist/direct-touch-debug/iMirror.exe` and `dist/portable`.
The first two 30-second attempts reported `No iPhone is connected`; those are
failed discovery attempts, not performance results. A fresh hardware baseline is
pending reconnection. Latest pre-change native logs show 1180x2556 H.264 through
Microsoft H264 Video Decoder MFT; sampled decode times were 6.4–8.0 ms. These are
historical log samples, not a fresh source-FPS benchmark or percentile distribution.

Old mouse path: preview WM_MOUSEMOVE -> screen-coordinate delta -> 32-entry shared
command FIFO -> sensitivity -> subscriber enumeration -> synchronous GATT notify.
Numeric latency/rate results must remain unmeasured until an instrumented physical
run actually occurs. A successful notification is not end-to-end pointer latency.

After reconnecting, the unchanged release completed a 30.056-second USB/render
baseline: **1180x2556, 56.5880 source FPS, 56.4474 submitted FPS, hardware decoder**,
1,611 source frames and 1,606 render submissions. Final sampled decode time was
14.4557 ms (a single sample, not a percentile). The run overlapped compilation;
CPU contention is a recorded limitation. Raw output:
`work/native-ui-milestone/mirror-before-connected.json` and `.samples.jsonl`.

## Work sequence

- [x] Preserve current source and rollback tag.
- [x] Record fresh mirror baseline with unchanged release.
- [x] Remove all Direct Touch production handlers/state/commands/docs.
- [ ] Instrument existing relative path with monotonic timestamps before replacing it.
- [ ] Raw Input receiver and global Ctrl+Alt+Q independent of BLE/UI work.
- [ ] Single accumulated movement slot; ordered button/wheel/key transitions.
- [ ] Cache GATT recipient and characteristic; negotiated connection pacing.
- [ ] Shared ControlManager; 100% initial baseline, persisted sensitivity.
- [ ] Native compact Windows 11 UI, themes/high contrast/DPI/accessibility.
- [ ] Connection / Control / Display / Advanced settings; diagnostics opt-in only.
- [ ] Official Microsoft runtime redistribution, EXE/MSI/Setup/portable.
- [ ] Full fmt/Clippy/tests/release checks without exceptions.
- [ ] Before/after mirror comparison and user's seven physical control tests.
- [ ] Logical implementation commits and final honest validation report.

Reference pacing audit: windows-ble-hid commit
`9a4f45129779ec3bef368ea4ffcdcde795a57b65` uses a signaled accumulator and reads
BluetoothLEDevice.GetConnectionParameters().ConnectionInterval (units 1.25 ms).
Its comments warn that notify completion can reflect queueing, not transmission;
unpaced submissions can create an invisible Bluetooth-stack movement backlog.
Therefore GATT completion speed alone is not a safe report cadence. Movement will
be paced from negotiated link timing with a measured fallback and stale expiry;
button transitions remain ordered and bypass waiting behind movement.
