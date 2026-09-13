# WDA latency and Wireless Quality validation — September 12, 2026

Later runtime management and reopen acceptance are recorded separately in
[WDA_MANAGED_RUNTIME.md](WDA_MANAGED_RUNTIME.md). Measurements below describe
the earlier candidate and are not a benchmark of that later build.

Test candidate: `dist/wda-latency-20260912/iMirror.exe`, 3,268,096 bytes.
SHA-256: `31b534857f3dd54b0a80ffe57940b7cd2e76128ba05d36158a1b50e9c0b8e0fb`.
Phone: iPhone15,4, iOS 27.0. Video uses Wireless; WDA uses the existing USB
tunnel and loopback forwarder. This is an engineering candidate, not a signed
installer or a clean-Windows release.

## Failure reproduced before this change

After the initial successful click, the user reported that mirror clicks stopped
working. Two normal iMirror processes were open. The second could not register
Ctrl+Alt+Q (Windows error 0x80070581); its diagnostics reported backend 0, not ready,
and the persisted Control setting was off. The WDA server itself still answered
and returned geometry 393x852 in 315.65 ms. That window's mouse-down readiness
gate therefore prevented dispatch; a working `/status` endpoint did not mean the
application had an active controller.

The user closed both instances. Re-enabled the already selected Wireless/WDA
configuration with a backup and opened one instance. It reported backend 2,
ready, a registered emergency shortcut and no error. The user then confirmed
clicks worked, but felt delayed. This is evidence of recovery, not unattended
WDA lifecycle reliability.

## Measured avoidable work and resulting change

Previously every tap called `GET /window/size` before `POST /wda/tap`, although
Control already obtained geometry at connection and refreshes it on video-format
change. Five read-only geometry measurements were 173.668, 140.752, 241.492,
153.386 and 116.378 ms (average 165.135 ms). A separate phone-side XCTest tap
activity took 440.566 ms. These different measurements must not be summed into
a claimed before/after click benchmark.

The client now caches geometry per WDA session. Explicit geometry refresh still
fetches it; request failures invalidate the cache. A definite invalid-session
rejection may recreate the session, but coordinates are revalidated against fresh
geometry before replay. A size change rejects the old mapped point. Ambiguous
timeouts never replay input. Successful WDA connection also clears stale error
text. No input coordinates/text/session identifiers are included in timing data.

Diagnostics include request counts and the most recent 64 software tap-dispatch
durations. These exclude the Windows UI queue and physical display latency.
Metrics reset when a new WDA client is connected.

The user confirmed functioning clicks and a slight subjective improvement.
One sample window contained 54 tap requests, zero failures, only two geometry
requests (connection/format setup), median 597.097 ms and mean 667.020 ms dispatch
time. Thus repeated geometry queries were removed in real operation. This is
not an exact paired end-to-end improvement measurement, nor instant input.

## Duplicate launch prevention

A Windows exclusive file handle in the user's iMirror data directory now admits
only one normal desktop instance. Subsequent launches activate the existing
visible iMirror window instead of starting another control worker. The lock is
released by the OS on process exit; the empty file may remain. CLI benchmarks
and smoke modes remain independent. An actual second normal launch exited 0,
the first process stayed live and exactly one iMirror process remained.

## Wireless clarity comparison

The earlier sharp/smooth Bluetooth-control setup used USB video at 1180x2556.
Bluetooth did not carry its video. Wireless Auto requested 1920x1080 and the
phone supplied portrait 498x1080.

With the app closed, backed up its settings and changed only the existing
`quality` value from `Auto` to `Quality`. This preset requests 2560x1440 / 60 FPS.
The same EXE then received actual 664x1440 video; the user confirmed sharper and
smooth live video while scrolling directly on the phone. No queue recovery or
decoder runtime failure was found in that run. The Quality preference is retained
for this user; the global default is unchanged. This does not validate sustained
60 FPS or actual display FPS. No capture/decoder/renderer source changed.

## WDA recovery during the quality check

The phone runner exited with EOF/lost-testmanagerd. Restarting it produced a
working phone-side server, but the old Windows forwarder still could not reach
it. Restarting that exact forwarder restored loopback `/status`. The user selected
Connect WDA in the existing app and confirmed control worked again with the
Quality video still live. The precise runner exit and old-forwarder failure causes
are not established. These helpers are not yet automatically supervised by iMirror.

After reconnect, one window contained 11 HTTP taps with no failures and one
geometry request. Software dispatch median was 614.651 ms, mean 646.891 ms.
These counts are not proof that every request was independently observed on the
physical phone; the user's explicit successful-click confirmation is the physical
evidence. WDA remains substantially slower than the verified Bluetooth Mouse path.

## Checks and remaining limits

- `cargo fmt --check`: PASS.
- `cargo clippy --all-targets --all-features --locked -- -D warnings`: PASS.
- `cargo test --locked`: 63 tests PASS.
- `cargo build --release --locked`: PASS.
- New tests cover cached-request counts, rotation refresh, session expiry with
  unchanged/changed geometry, no timeout replay, bounded metrics and instance-lock
  release. Mock HTTP is used only in these software tests.
- No native media, USB, AirPlay, BLE or Raw Input source changes in this phase.
- Runtime files other than the EXE match the previous Wireless candidate, and
  PE dependency checks found no new DLL imports. Launch used a system-only PATH.
- Physical rotation after caching, drag, keyboard, 30-minute stability, helper
  lifecycle, clean Windows installation and signing remain unvalidated.

Local numeric evidence is in `work/wda-setup/wda-geometry-baseline.json`,
`wda-cache-physical.json` and `wireless-quality-acceptance.json`. Private logs,
pairing records and signing material stay outside Git.
