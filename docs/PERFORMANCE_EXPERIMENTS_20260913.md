# Performance experiments — 2026-09-13

Work in progress, not a stable release. Branch `experiment/performance-20260913`.
Base: `checkpoint/wda-fast-input-working-20260913`, commit
`bfa4784a88adf793052b44ad758fa9d7ec7994ba`.
The original `dist/wda-fast-input-20260913/iMirror.exe` remains unchanged:
`C577805DB7CD72AC01A52AB2D7E83EDD9F450F9C342941C1957FBE32BAF54DB0`.
No merge, stable tag change or upload is authorized before manual acceptance.

## Measurement and attribution

The first measurement-only suite remains in ignored
`work/usb-pacing-20260913/run-001`, including native CSV, PresentMon CSV, PTS
correlation and a Vietnamese report. It used checkpoint source plus observation
hooks, not the older click-trace EXE. `scripts/analyze-usb-spikes.py` now correlates
every displayed interval over 25/40/50 ms through source PTS, source observation,
decode, mailbox, render, Present and ETW. Missing source frames between displayed
frames are classified separately. First threshold crossing is descriptive; it
does not establish a single causal owner. Receive-to-display excludes phone
capture/encode and any USB time before the host observation.

The expanded profile suite `run-profile-001` completed A/B/C, each over 60 s,
on the real iPhone. Binary:
`dist/imirror-performance-experimental-20260913/profile-baseline/iMirror.exe`,
SHA256 `9EC712EECD3EC5EC475E807BB7D505C25D47EB8034C5C2223AB30E5494AA3C3C`.
H.264 1180x2556, hardware MF, child-HWND D3D11, viewport 564x1222, DPI 144,
VSync on and no render FPS cap. WDA/BLE were not started by this harness.

Expanded observations include DXGI lookup, staging creation, GPU copy submission,
Map wait, CPU allocation/copy, MFT input/output calls, Annex-B conversion,
renderer upload, USB reads, packet batch parsing, render start, and process RAM.
Hooks do not issue GPU queries or force GPU flushes. Recorder allocation is
bounded; export happens outside the measurement interval. Existing and expanded
instrumentation have not had a full overhead A/B study, so run-to-run differences
are not attributed solely to product changes.

| Profile | Source FPS | Display ETW FPS | Display P95 ms | Display P99 ms | 1% low |
|---|---:|---:|---:|---:|---:|
| A static | 59.927 | 50.965 | 41.722 | 55.753 | 16.151 |
| B continuous scroll | 56.567 | 49.303 | 41.736 | 54.920 | 15.805 |
| C fast scroll | 57.450 | 57.384 | 25.146 | 49.961 | 19.418 |

A/B are materially worse than C with the same policy. Do not conceal this or
use only the favorable case. A/B had 146/118 decoded frames not accepted after
the tail; C had none. The static source rate also changed from the older suite's
41 FPS to about 60 FPS, so these are not identical source workloads. Repeated
controlled comparisons are needed before retaining an optimization.

Map dominates decoder CPU/API wall time: A average 6.757 ms, P99 13.098 ms;
B average 7.047 ms, P99 12.500 ms. CPU memcpy averages 0.898/0.887 ms; allocation
0.027/0.028 ms. Staging resources were reused (zero creation events in the measured
windows). GPU-copy submission is about 0.004 ms, which does not measure GPU
completion. Detailed per-frame data, all stage percentiles and RAM samples are
in the suite analysis files.

## Experiment ledger

| ID | Hypothesis | Change | State / decision |
|---|---|---|---|
| P0 | Current counters do not locate copy/wakeup spikes | Add bounded QPC observation and ETW correlation, no pipeline policy change | A/B/C completed; measurement only |
| M1 | GPU readback/upload creates avoidable serial latency | Opt-in shared GPU handoff, nonblocking keyed mutex, adapter check, CPU fallback; original Present policy | REJECT standalone: lower CPU/mean latency but controlled A repeat lost 177 decoded frames and worsened P99 |
| M1b | Repeated shared-resource import/view creation adds work | Cache 16 imports without retaining producer pool slots | REJECT standalone: cheap retry loop reached about 3000 attempts/s in smoke; stopped before A/B/C |
| M2 | DO_NOT_WAIT retries may add wakeups/jitter | CPU output, same VSync, Present flags 0 | A/B/C completed; comparison candidate only, not a universal pacing win |
| M3 | Cached GPU handoff can work with a blocking Present instead of rapid retries | Exact M2 EXE, enable GPU handoff/cache; unchanged VSync and queues | KEEP as opt-in experiment for manual evaluation; not stable/default. B/C improved, A still loses 45 decoded frames |
| W1/T10 | Shorter contact can reduce W3C tap time without missed taps | 50/30/20/10 ms | Separate 400-tap run completed, 100/100 PASS each; 162 earlier visual observations. 10 ms kept opt-in, default remains 50 ms |
| W2 | Rust preparation, queue and response processing may add latency | Monotonic request/command observations and opt-in real Calculator ROI fixture | Measurement only; no transport rewrite |
| W3 | XCTest persistent state might support streamed touch | Read WDA synthesizer and private-header interfaces | RESEARCH ONLY; declarations are not validated continuous-touch semantics |

M1 is minimal-copy GPU handoff, **not absolute zero-copy**: decoder GPU surface
is copied once to a shareable GPU texture. It is enabled only by
`IPHONE_MIRROR_EXPERIMENT_GPU_HANDOFF=1` in the test launcher. Existing CPU readback
remains the default. M1 preserved Present flags; M2/M3 separately opt in with
`IPHONE_MIRROR_EXPERIMENT_BLOCKING_PRESENT=1`. VSync, frame queue policy, quality,
resolution, source timestamps and BLE/Raw Input algorithms are unchanged. Consumer failure
requests CPU fallback for that decoder lifetime; the decoder does not wait on a
cross-device consumer. The old source comment documents a previous keyed-mutex
rotation failure, making rotation and fallback acceptance essential.

The renderer's imported-resource cache is enabled only with the blocking-Present
experiment, avoiding the M1b retry loop. Experiment permission is explicitly
passed only by USB capture/preview construction; wireless and legacy constructors
do not opt in. Both environment switches are off by default. There is no
waitable-swapchain rewrite, busy spin, priority boost, or high-resolution timer
change. A failed GPU consumer disables handoff for that decoder lifetime and
future outputs use the original CPU path. Fault-injected adapter/device-loss
fallback remains unvalidated on physical hardware.

## USB results and limitations

Every completed case below is at least 60 seconds. P50/P95/P99/max are **unique
ETW displayed-frame intervals**, not accepted Present attempts. Source identity
is PTS identity, not a hash of identical screen contents. No encoded queue drops
were observed in these suites. Counts at exact window boundaries can differ by
one frame across receive/decode/display; M3 C's display rate slightly exceeding
source does not mean a new source frame was generated.

| Suite/case | Source FPS | Decoded FPS | Display FPS | Avg ms | P50 ms | P95 ms | P99 ms | Max ms | 1% low |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Original A | 41.016 | 41.016 | 40.916 | 24.406 | 16.706 | 53.097 | 83.444 | 108.918 | 10.381 |
| Original B | 57.161 | 57.161 | 57.044 | 17.531 | 16.691 | 33.379 | 50.083 | 66.788 | 17.765 |
| Original C | 59.013 | 58.996 | 58.896 | 16.977 | 16.688 | 25.212 | 50.072 | 75.029 | 18.875 |
| M2 A | 58.971 | 58.954 | 58.705 | 17.034 | 16.686 | 25.245 | 58.431 | 100.124 | 15.729 |
| M2 B | 59.286 | 59.286 | 59.120 | 16.914 | 16.686 | 25.177 | 58.421 | 91.781 | 16.071 |
| M2 C | 58.270 | 58.286 | 58.220 | 17.180 | 16.691 | 25.346 | 43.867 | 65.681 | 19.040 |
| M3 A | 40.858 | 40.858 | 40.044 | 24.961 | 16.708 | 52.665 | 75.124 | 108.535 | 10.688 |
| M3 B | 58.113 | 58.113 | 58.030 | 17.232 | 16.693 | 25.072 | 41.726 | 58.421 | 21.110 |
| M3 C | 58.460 | 58.460 | 58.477 | 17.103 | 16.692 | 26.028 | 34.677 | 67.387 | 22.661 |

Original evidence: `work/usb-pacing-20260913/run-001`. New suites are in the
same parent directory: `run-m2-blocking-present-001`, `run-m3-gpu-blocking-001`.
This is sequential human-driven scrolling, not deterministic replay. The source
varies, especially static A (41 vs 59 FPS); neither those A runs nor any two
different source workloads are treated as a controlled speedup percentage.

| Suite/case | Decode avg/P95/P99 ms | Renderer avg/P95/P99 ms | Rx-to-display avg/P95 ms | CPU avg % | WAS_STILL_DRAWING | Decoded never accepted by tail |
|---|---|---|---|---:|---:|---:|
| Original A | 8.295 / 8.955 / 30.323 | .633 / .824 / 2.999 | 24.720 / 47.265 | 1.086 | 403 | 0 |
| Original B | 9.462 / 13.401 / 18.098 | .588 / .717 / 1.635 | 24.756 / 32.799 | 1.440 | 353 | 0 |
| Original C | 8.108 / 11.121 / 16.500 | .660 / .900 / 3.501 | 24.044 / 34.596 | 1.752 | 725 | 0 |
| M2 A | 7.716 / 8.737 / 14.064 | .808 / 2.571 / 6.716 | 24.037 / 39.097 | 1.323 | 0 | 0 |
| M2 B | 7.595 / 8.863 / 13.539 | .863 / 2.274 / 6.842 | 24.272 / 37.942 | 1.450 | 0 | 0 |
| M2 C | 8.507 / 11.115 / 14.867 | .648 / .946 / 2.241 | 23.278 / 29.092 | 1.600 | 0 | 0 |
| M3 A | .551 / .782 / 1.197 | 1.021 / 2.319 / 21.528 | 20.461 / 31.877 | .550 | 0 | 45 |
| M3 B | .562 / .712 / .976 | .870 / 4.051 / 7.746 | 22.019 / 27.273 | .710 | 0 | 2 |
| M3 C | .612 / .828 / 1.804 | .813 / 3.680 / 7.117 | 21.306 / 26.594 | .738 | 0 | 0 |

Processing timings are CPU/API wall time, including Present blocking, not GPU
timestamp execution time. CPU is normalized to this host's 12 logical CPUs.
WAS_STILL_DRAWING becoming zero follows changed Present flags and by itself is
not proof of better latency. Successful duplicate PTS presentations were zero
in these measured cases. Original retry gaps averaged .845–.955 ms, P99
6.277–7.329 ms. M2/M3 have no retry gaps to measure.

Controlled topmost static repeat, same EXE (`repeat-a`, SHA256
`EEBE56BDC2D99847842CF1FBDE546129584C1A39BA8FDB5AACD8C8E5287F683E`),
only GPU switch changed: CPU source/display 59.244/59.078 FPS, P99 58.451 ms,
1% low 16.080, CPU 1.966%, mailbox loss 0. GPU M1 source/display 59.312/56.331,
P99 66.706 ms, 1% low 14.186, CPU .911%, mailbox loss 177. This is why M1 was
not accepted even though mean receive-to-display fell 25.141 -> 19.900 ms.
The earlier M1 A possibly occluded result is preserved separately, not used as
proof of GPU failure; the user was unsure whether another window covered it.

M2 and M3 used the exact same EXE, SHA256
`5AD1D56B1D62B5490BAA8AEEB9F994F551373C1CD1FB4FFF948D19C02ACC1166`,
with GPU switch 0/1. M3 cache hits A/B/C: 2413/3492/3516; consumer fallbacks 0.
Readback Map/CPU-copy observation counts are zero in M3. GPU handoff submit
average in C is .206 ms, cached import .00169 ms; these do not measure the GPU
copy duration. M3 A's 45 unpresented outputs (~1.8%) remain a release concern.

### Where spikes begin

Source itself is not perfectly periodic. Original C source PTS P99 is 33.282 ms,
arrival P99 38.552 ms, display P99 50.072 ms. M3 C PTS P99 remains 33.283 ms,
arrival P99 34.588 ms, display P99 34.677 ms. A at ~41 FPS has source PTS P95/P99
near 50 ms. A renderer cannot generate missing new source frames at 60 FPS.

Original C has 411/133/58 display intervals over 25/40/50 ms. Among >40 ms pairs,
96 first cross the threshold at decoded-mailbox publication, 25 at host source
observation, 4 in source PTS, 5 at Windows display; 3 span skipped source frames.
For >50 ms, 44 first cross at Windows display, 6 at decoded publication, 4 at
host observation, 1 at accepted Present, 3 span skipped source frames.

M3 C has 456/27/10 intervals over those thresholds. The >40 ms first crossings
are source PTS 12, host observation 8, Windows display 7. For >50 ms: Windows
display 5, host observation 4, render start 1. This supports **mixed source,
host-readback and display scheduling jitter**, not one exclusive root cause.
First crossing is descriptive, and earlier/later stage expansions can coexist.
USB host-observation jitter does not locate the stall uniquely on the phone,
cable, driver, or host scheduler. Packet parsing and Annex-B work are small
(M3 C avg .022/.008 ms); no evidence justifies rewriting CaptureSession queues.

### Resources and physical smoke

Mean working set/private bytes in MiB, M2 A/B/C:
169.862/201.668, 171.073/202.882, **31.572/204.197**. M3 A/B/C:
166.417/186.737, 121.267/188.013, 123.036/188.810. M2 C's working set was trimmed
by Windows (27.910–36.402 MiB), so comparing its working set directly to M3 as an
allocation regression is invalid. Private bytes are less affected by residency.
Original run-001 has no RAM observations; do not report zero RAM. These are
one-minute samples, not evidence of 30-minute leak freedom. GPU memory/time,
handle growth, and full system power were not measured.

The user physically confirmed correct/live M1/M3 video. On the later USB-scoped
normal-UI candidate (`candidate-lab/iMirror.exe`, SHA256
`811DD5D972B299CD96606F9386E1C90D45DCA74B42581CEAD5F4B949EB87AFE3`),
they confirmed rotation, fullscreen, resize, minimize/restore and button
Disconnect/Connect were normal. A subsequent no-device log coincided with the
user unplugging/not replugging the cable; do not attribute it to an optimization.
Automatic cable reconnection, GPU fault fallback, BLE, keyboard and Raw Input
acceptance are not newly proven for the final experimental EXE by that smoke.

## WDA findings and limits

The old 567 -> 429 ms comparison had five taps per variant, not a 100-sample
latency distribution. Do not invent its P95/P99 or input-to-visual latency.

Pinned WDA source inspected at `3e8aa7de81f254dbb0876baa9e9173c16b55b3a0`:

- [Touch routing](https://github.com/appium/WebDriverAgent/blob/3e8aa7de81f254dbb0876baa9e9173c16b55b3a0/WebDriverAgentLib/Commands/FBTouchActionCommands.m)
- [W3C synthesizer](https://github.com/appium/WebDriverAgent/blob/3e8aa7de81f254dbb0876baa9e9173c16b55b3a0/WebDriverAgentLib/Utilities/FBW3CActionsSynthesizer.m)
- [Event record interface](https://github.com/appium/WebDriverAgent/blob/3e8aa7de81f254dbb0876baa9e9173c16b55b3a0/PrivateHeaders/XCTest/XCSynthesizedEventRecord.h)

Each request constructs event paths and a complete event record; the existing
daemon proxy waits for synthesis completion. Pointer-up depends on a path in
the same chain. The standard action route does not implement a persistent
touch session across HTTP requests. The private header includes
`beginsPersistentState` / `endsPersistentState`; declarations alone do not prove
working touch continuity or safe cancellation on this phone. No micro-swipe
approximation has been implemented.

User confirmed no local Mac/Xcode. GitHub macOS runners are a possible separate
build route, but this private repository's CI can consume billed minutes; no
remote job/push was initiated. A patched runner would still need user signing
and installation. Continuous touch is not declared impossible, but no custom
runner or continuous semantics have been built/physically validated here.
Existing Windows-managed WDA and tap benchmarks remain available. No credentials
are requested or logged.

New `HttpTiming` explicitly distinguishes preparation, headers-ready, body-ready
and completion. Headers-ready is **not** first-response-byte timing. The waiting
interval combines forwarding, HTTP, WDA routing and XCTest until a server-side
probe is available. No subtraction of a `/status` round trip will be presented
as an exact XCTest duration.

The inspected `FBRunLoopSpinner.m` uses a 100 ms run-loop interval in
`spinUntilCompletion`; `FBXCTestDaemonsProxy::synthesizeEventWithRecord` calls it.
This is a candidate source of completion-tail delay, not measured evidence that
changing it reduces input-to-visual latency. No global interval/transport patch
was applied. A useful next runner experiment must separately timestamp parsing,
synthesis submission, callback and response before changing the wait strategy.

### Interrupted tap run / power-state confound

`work/performance-20260913/wda-taps-004` attempted 214 primary taps: 213 real
Calculator results verified as 1, one unknown because the result-state GET ended
with an incomplete HTTP message. No observed wrong/missed result; this does not
make the unknown a PASS. Each contact has only 53/54 samples, below the 100 gate.
All ROI observations exist, but only 5 PASS trials have an ETW displayed ROI
match. The report is explicitly invalidated for acceptance.

Windows System log proves Modern Standby/Idle Timeout during this measurement;
the renderer also reported 1x1 while the display was inactive. User confirmed
USB remained plugged in and Calculator/iMirror were visible when asked. Do not
claim the user unplugged or minimized it. The tunnel subsequently failed and
did not become ready until cable reconnection. PresentMon's stop supervisor
could not find its former session; CSV finally flushed on timed termination.
Raw data, collector error, and power events are retained.

A repeat fixture now holds a thread-bound `SetThreadExecutionState` guard using
SYSTEM_REQUIRED/DISPLAY_REQUIRED/CONTINUOUS for the measurement only, restoring
the previous state on scope exit. It does not modify power policy, block manual
sleep/lock, or alter production iMirror startup. ROI arming checks a visible,
non-minimized owned viewport, and observations record actual render dimensions.
This fixes a measurement condition, not WDA latency. Benchmark repetition is
pending; no new tap contact duration has been accepted.

Repeat preparation: `wda-awake-lab/iMirror.exe` SHA256
`F2AC493E52810469C93F7CED1436795972AE2699A5BE1B7176DC0695FFDE34AD`.
The 8-second collector-only preflight saw 58 Present records / 51 displayed
records; these include duplicates and are not a source-FPS result. Attempts
005/006 stopped before scored taps because the GUI's live-video gate was false
despite WDA Ready and the user's visual connection confirmation. Native wireless
logs included encoded-packet-limit recovery. No additional Modern Standby was
found for attempt 005. The live-state discrepancy requires a current GUI
diagnostic snapshot; do not remove the gate or count stale video as validation.
Any preparation clear/zero tap is separate from scored trials. Attempt 006 also
had a launcher argument typo; it was fixed before launching any input process.

The user subsequently supplied a current Copy Diagnostics snapshot from PID
20760: active/state 4, software-decoded wireless 664x1440, 57.535 source FPS,
16,488 source frames and WDA Ready. A fresh local arm passed the video gate and
rejected only the old ROI reference. This proves current liveness, not liveness
at the previous attempts' timestamps. A separate no-input inspection then found
Calculator was not foreground. The existing WDA session was reused to activate
only `com.apple.calculator`; no session recreation or arbitrary-app tap.
Attempt 007 used a fresh fixture/reference. It stopped after 163 verified taps
when wireless hit `encoded_packet_limit` (16 packets / 1603 bytes) and entered
keyframe recovery. No Modern Standby was observed in that run. All 163 real
Calculator results passed; 162 had a matched changed ROI displayed by ETW. The
missing observation remains missing, not assigned its HTTP completion time.

| Contact ms | Real PASS / attempts | HTTP avg / P50 / P95 / P99 ms | ETW result samples | Dispatch-to-result-display avg / P50 / P95 / P99 ms |
|---:|---:|---|---:|---|
| 50 | 41 / 41 | 476.325 / 471.156 / 621.991 / 632.434 | 41 | 323.167 / 313.117 / 459.318 / 478.845 |
| 30 | 41 / 41 | 483.071 / 483.244 / 592.023 / 604.281 | 41 | 304.776 / 305.449 / 420.237 / 429.585 |
| 20 | 41 / 41 | 486.039 / 481.957 / 629.611 / 742.273 | 40 | 302.477 / 296.210 / 437.194 / 564.642 |
| 10 | 40 / 40 | 481.386 / 486.153 / 600.188 / 625.980 | 40 | 294.887 / 294.149 / 421.465 / 451.349 |

These are **preliminary distributions below the 100-per-contact gate**, and
show the Calculator result region, not the earlier key highlight. Input starts
at generated WDA dispatch, not a physical mouse switch; display is Windows ETW,
not phone photons. `/status` averaged 4.193 ms (30 requests); request preparation
averaged .019–.021 ms. HTTP completion trails the result on screen, so optimizing
only a completion wait may not improve the user's first visual response.

Inspection, without modification, confirms wireless `EncodedFrameQueue` has a
16-packet streaming bound and 250 ms age limit. On rejection `EncodedSession`
clears pending compressed frames, advances generation, sets Handshaking and
ignores dependent packets until a keyframe. No explicit keyframe request occurs
in that failure function. This explains why a stream can remain connected while
the live gate becomes false. The original producer of the burst/stall is not
yet isolated. Queue limits, wireless recovery, and video code were not changed
to make the tap test pass.

Attempt 008 completed a separate 100-per-contact HTTP/real-Calculator run with
the same probe and Windows awake guard, **without ROI/ETW** because wireless had
entered recovery. It is not a healthy-mirror latency or stability test and must
not be combined with attempt 007 as one end-to-end distribution. All 400 primary
taps changed the verified result 0 -> 1; zero missed/wrong/unknown results.

| Contact ms | Real PASS | HTTP avg ms | P50 ms | P95 ms | P99 ms | Max ms |
|---:|---:|---:|---:|---:|---:|---:|
| 50 | 100/100 | 419.438 | 394.516 | 535.136 | 620.604 | 662.736 |
| 30 | 100/100 | 423.509 | 394.293 | 540.420 | 610.341 | 791.037 |
| 20 | 100/100 | 414.534 | 393.914 | 538.317 | 613.542 | 684.888 |
| 10 | 100/100 | 415.016 | 392.042 | 533.208 | 545.200 | 547.200 |

HTTP median gain at 10 ms is only 2.473 ms, mean 4.422 ms. This is not a large
HTTP speedup and does not establish <300 ms HTTP. The tail was lower in this run;
100 samples do not guarantee a persistent P99 improvement. The earlier visual
sample had mean 323.167 -> 294.887 ms and median 313.117 -> 294.149 ms (41/40
observations); that suggests earlier visual response but remains below the
100-per-contact visual gate. No universal 100% reliability claim is made from a
single Calculator fixture.

**T10 decision: KEEP only as an opt-in manual-test candidate.** The application
factory honors `IMIRROR_EXPERIMENT_WDA_SHORT_TAP=1`, selecting a fixed 10 ms
complete DOWN/pause/UP W3C tap after the existing session tuning checks. Ordinary
constructors/launches stay at 50 ms. Native tap fallback, session validation,
geometry cache, no-replay-on-timeout, HTTP pooling and swipe logic are unchanged.
Diagnostics report the applied contact. Loopback tests validate complete report
bodies for both 50/10 ms and native fallback in both modes. The newly integrated
EXE has not yet received the user's physical acceptance.

The same pasted snapshot provides one actual GUI tap and swipe software trace:
mouse-up-to-enqueue .0142/.0706 ms; queue .0154/.0150 ms; dispatch-to-HTTP
.0134/.0162 ms; request preparation .0154/.0171 ms. Tap HTTP total 718.171 ms,
swipe 979.884 ms. Mouse-down-to-enqueue 74.068/90.559 ms includes human hold.
These are one sample each, without physical outcome/visual-time confirmation;
do not create P95/P99 claims from them. They support measuring server/transport
waiting before rewriting the already short local queue.

## Prepared runner-side observation patch

`experiments/wda-timing/runner-timing.patch` adds an explicit timed actions route
and handler/event-construction/daemon/callback/wait timestamps to pinned WDA.
The three base blobs match upstream and `git apply --check` passes. The macOS
build script passed shell syntax checking. **Objective-C compilation, signing,
installation and physical tests have not run.** No Windows product path enables
this endpoint. No faster wait, new transport or continuous gesture has been
implemented. The directory contains build/validation instructions and scope
limitations for a future signed-runner experiment; no remote CI was initiated.

## Checks so far

Expanded profile: fmt, strict Clippy, 73 Rust tests and release build passed.
One initial Clippy native compilation failed in MSVC's `chrono` header with
split identifiers (`_Clock_cast_s`, `rategy`); reading the header showed the intact
identifier, and an unchanged-source retry passed. No workaround or modification
to the compiler installation or frozen wireless source was made.

M1, M2 and the USB-scoped candidate were built and tested as recorded above.
The awake GUI lab passed fmt, strict all-target/all-feature Clippy, 75 default
Rust tests and release build (`work/performance-20260913/wda-awake-lab-*.log`).
T10 integration also passed those four gates (`final-experimental-*.log`). The
optional fixture was compiled as an optimized example; its additional feature
tests and four offline ROI/ETW analyzer tests passed before T10 integration.
Final staging/import inspection passed for 83 EXE/DLL, with unchanged runtime
hashes and expected private/System32/API-set resolution. New EXE:
`dist/imirror-performance-experimental-20260913/iMirror.exe`, 3,453,440 bytes,
SHA256 `0FA2692831B65FB09109287325826821BD1E75AE286B95BCB8DD22F30D4BFDE4`.
Unsigned, not launched to replace the current GUI. Manual exact-binary acceptance,
clean-machine testing and release remain unproven. WDA's 19 all-feature tests
were also rerun successfully after T10 integration.

Code is preserved in local experimental commits `c17b5b0` (USB/native measurement)
and `eee5b9e` (WDA timing/T10). No push, main merge or new checkpoint tag. Full
Vietnamese results are in `PERFORMANCE_REPORT_20260913.vi.md`.
