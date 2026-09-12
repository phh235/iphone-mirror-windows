> Historical snapshot; see [current validation](../VALIDATION.md) and the [history index](README.md). This record is not current setup guidance.

# Product hardening — hardware acceptance gate

Checkpoint: `checkpoint/native-ui-ble-working` at `7549f09` on
`work/native-ui-raw-input`. Four commits preserve the previous native UI, Raw
Input, control/settings and CRT staging implementation. An offline release build
from a separate Git archive of that tag passed, including rebuilding the native
core and USB helper; no untracked source was required.

## Changes before the hardware gate

- HID caches are tied to a subscription/session generation. CCCD callbacks,
  session transitions, protocol suspend/resume and notification errors invalidate
  the generation. An Active callback cannot revive an old client. Re-selection
  enumerates the current subscribers, checks protocol state and sends neutral
  reports before readiness is published.
- Invalidation releases local capture through the independent Raw Input thread,
  clears motion and queues neutral input. No local release waits for GATT.
  Remote delivery still depends on the Bluetooth link and cannot be guaranteed
  while disconnected. All normal/diagnostic notifications use the same cached,
  validated, bounded notification path.
- The movement accumulator/scaling/pacing algorithm is retained. The transition
  queue is capped at 128. Saturation cancels capture, stops new presses and
  supersedes the pending sequence with mouse-all-UP and keyboard-all-UP reports.
  Thus release state is preserved without retaining an unbounded action history.
- Safety-generated neutral reports are excluded from physical-input latency
  samples. A real button-up still contributes its physical event timestamp.
- Pairing guidance requires current STARTED advertising. WDA status is separate;
  Home requires a ready WDA connection and valid geometry. Home visibility changes
  trigger layout. Settings has native scrolling and focused-control reveal for
  small windows; existing pages and visual style remain.
- Control diagnostics create their parent directory before opening files. JSONL
  storage is limited to a 2 MiB current file and three 2 MiB archives. Previous
  oversized generated logs are discarded when applying the cap. File errors are
  exposed instead of silently ignored. No packet-by-packet file logging is added.
- Settings > Advanced > Copy Diagnostics exports app/Windows/device versions,
  video state/decoder/resolution/source FPS, Bluetooth subscription counts,
  readiness, software latency metrics and recent errors. Phone names, UDIDs,
  Bluetooth addresses, raw input text and pairing material are excluded.
- Obsolete automatic Move Right text is removed. Diagnostic movement still
  requires an explicit button press.

## Validation boundary

The earlier user-confirmed responsive pointer behavior belongs to the previous
preview EXE. It does not validate these changes. The previous neutral-capture
correction also needs acceptance in this exact release.

UI smoke tests use app-owned windows and explicit test-only DPI overrides;
they do not change Windows settings, start mirroring, advertise HID or send
phone input. They supplement, rather than replace, actual monitor-DPI and
physical iPhone testing.

No frozen media implementation, audio behavior or Valeria/09:41 configuration
is changed. No performance benchmark, 30-minute run, new wireless deployment,
installer build or clean-machine test belongs before physical acceptance.

## Required user acceptance

USB mirror; rotation; Control ON; Bluetooth reconnect; slow and fast mouse
movement; abrupt stop; click; drag; wheel; typing; Ctrl+Alt+Q; focus loss; device
disconnect/reconnect. Keep the complete staged folder beside the EXE. Report
each failure and use Copy Diagnostics when needed. Do not claim production
readiness from the software checks.
