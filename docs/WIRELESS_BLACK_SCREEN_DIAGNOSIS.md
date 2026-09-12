# Wireless black-screen diagnosis, September 12, 2026

Status: real-device failure reproduced and corrected; the user confirmed live
updates and one WDA click through the bounded-burst candidate. Long-run tests remain pending.

Phone: iPhone15,4, iOS 27.0. Wireless source: 498x1080 H.264.
User confirmed a black window with the second UxPlay timestamp candidate.
The unmodified app benchmark received 1,521 encoded frames and submitted zero
render frames. A separate instrumented copy of the production EncodedSession
then measured the following from the real phone:

| Counter | Result |
| --- | ---: |
| Assembled / submitted frames | 807 |
| Received keyframes | 1 |
| RTP/H.264 parser errors or drops | 0 |
| Negative source timestamp deltas | 0 |
| Encoded queue overflows | 1 |
| Decoder configurations / calls | 1 / 1 |
| Decoded frames returned | 1 |
| Decoder exceptions | 0 |
| Decoded frame rejected after generation change | 1 |
| Later frames rejected while waiting for keyframe | 803 |

The instrumented copy uses the actual H.264 assembler and native decoder. It adds
atomic counters to an otherwise unchanged EncodedSession copy. It does not
attach a renderer, store source images or send WDA input. The counter experiment
is diagnostic evidence, not a successful end-to-end mirroring test.

## Failure path

`vendor/iphone-mirror/src/Core/src/Capture/EncodedSession.h` reserves three
encoded-packet slots. While the worker initializes MF and decodes the first
keyframe, the receive thread fills those slots. `submit()` clears the queue,
increments `generation_` and sets `waiting_for_keyframe_` on overflow. Once
`decode()` returns a valid frame, `run()` detects the changed generation and
discards it. The next 803 non-keyframes are rejected; no second keyframe arrived
in this run. No image reaches `latest_` or the renderer.

The generation check protects against stale data after reset. Removing that
check alone would only expose a stale first image and leave ongoing video stuck.
Arbitrarily dropping compressed P-frames also cannot preserve decoding.

## Implemented follow-up, explicitly authorized by the user

The queue now preserves the initial keyframe and dependent encoded packets
during initialization, bounded by 32 pending packets, 16 MiB of pending payloads
(including SPS/PPS), and 500 ms of receive age. The observed cold configuration
took about 132 ms; the 500 ms budget provides headroom, with 32 slots covering
roughly that interval at a 60 Hz source. This is a cap, not an added wait.

After the first decoded frame, the startup queue drains in order. Only once
sixteen or fewer packets remain does it compact into the streaming queue.
Normal streaming also rejects a backlog older than 250 ms. Expiry/overflow keeps
the reference-frame recovery rule and now reports a Wireless video error with
reconnection guidance. It does not fabricate an IDR or remove reset protection.
No protocol keyframe-request mechanism has been added; after genuine packet loss
or sustained overload a new keyframe or Screen Mirroring reconnect may still be
required.

Startup slots release consumed payload allocations; normal streaming reuses
a bounded pool of packet buffers. The 16 MiB cap describes queued payloads, not total process
memory: one in-flight packet, cached buffers and decoder/GPU allocations are
additional. Generation validation and frame publication now share the queue
lock with reset/stop, preventing old decoded output from being published after
reset. Control, UI and rendering never acquire this queue lock.

The deterministic queue test stalls consumption while 20 dependent packets
arrive, checks the first keyframe survives, verifies ordered draining before
restoring the streaming limit, accepts a burst of 16 dependent packets, tests count/byte/age rejection, and runs 100
reset/wrap cycles. All six native CTest groups pass, including the recorded
encoded fixture. These software tests do not establish live iPhone acceptance.

This change belongs only to Wireless EncodedSession and its focused tests.
USB CaptureSession, QuickTime, MF implementation, D3D/DirectComposition, BLE,
Raw Input and WDA remain unchanged. The user explicitly approved this separate
Wireless queue correction. `fmt`, Clippy with warnings denied, 56 Rust tests and
release build pass. Candidate: `dist/wireless-startup-fix-20260912/iMirror.exe`,
3,259,392 bytes, SHA-256
`6c9d412596ec4a15126a436b023c0ea13f46bfb7f1b864fcd469a1004ba0c1d3`.
The helper and remaining runtime match the preceding clock-corrected stage.
This first startup candidate rendered initially, then froze; it is a failed
physical test. Its 200 ms samples advanced from 15 to 27 to 39 received frames
while render submissions stopped at 18 and the queue-recovery error appeared.
The benchmark's final empty error field is not a PASS: it only checked whether
any frame had decoded earlier and missed the later stall.

The first candidate restored the original three-packet streaming limit. That
limit remains too restrictive for the observed source delivery and decoder
scheduling. The current correction permits 16 compressed streaming packets,
still at most 16 MiB and 250 ms receive age, without waiting for a minimum fill.
The decoded/latest-frame renderer remains unchanged. Recovery logs distinguish
packet, byte and age limits; one-second decode summaries record publication
count, pending bytes, startup/streaming high-water marks and processing age.
The physical results of this follow-up are recorded below.

Follow-up EXE: `dist/wireless-burst-fix-20260912/iMirror.exe`, 3,260,416 bytes,
SHA-256 `6c73b777548caeb5c1f97fee77d6c5431a8d050cab04a84ea1e4c0a179baa2cf`.
All Rust checks and six native CTest groups passed for this candidate. It is a
hardware-test build, not an installer or a public release.

## Physical result of the burst candidate

The user confirmed continuous updates. During the 120-second run, 1,285 encoded
frames arrived; periodic logs counted at least 1,284 published decoded frames.
No queue recovery or decoder runtime failure was logged. Peak pending encoded
packets were 4 at startup and 10 while streaming, confirming that 3 was too low
for this real run. Resolution was 498x1080; the last sampled decode time was
2.518 ms. Whole-run source rate includes long static-phone intervals and is not
a controlled 60 FPS capability test. Render submissions are not unique displayed
frames. Hardware acceleration was not independently confirmed for Wireless.

This is short-run live-video acceptance only. Sustained overload/loss recovery,
30-minute stability and USB regression with this exact binary still require
separate validation. The WDA runner had exited
with an EOF/lost-testmanagerd error; it was restarted using the same existing
tunnel and signed runner, and the native UI now reports WDA ready.

The same EXE was then opened in its normal native UI with Wireless video and WDA
control over the existing USB tunnel. The user reset Calculator to zero and used
the Windows mouse to click key 1 on the live mirrored image. They explicitly
confirmed that the physical iPhone changed to 1. This verifies one integrated
absolute WDA tap, in addition to the earlier standalone HTTP tap. No second tap,
drag or keyboard command was sent by the agent. Integrated tap duration was not
measured; do not reuse the standalone request's timing for this UI test.

WDA remains optional/advanced, with the signed phone-side runner and separate
local tunnel/forwarder processes. This is not unattended WDA lifecycle validation,
nor a claim that QuickTime USB video and USB WDA work together on this host.

Local numeric evidence: `work/wda-setup/wireless-black-screen-counters.json`.
Private diagnostic run: `%LOCALAPPDATA%/iMirror-WDA/decode-queue-probe/`.
Neither signing material nor phone identifiers belong in this report or release.
