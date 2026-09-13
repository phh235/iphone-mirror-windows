# WDA latency tuning — September 13, 2026

This changes the optional WDA client only. USB, Wireless video, decoder,
renderer, BLE, Raw Input and production UI remain unchanged. The saved Wireless
Quality preference is retained.

## Tap comparison on the real phone

The preceding click-trace candidate dispatched about 0.022 ms after native
mouse-up, then spent 1068.205 ms in WDA for one captured tap. The Windows queue
was not the main measured delay.

With user approval, sent exactly ten Calculator key-1 taps, alternating these
profiles through one HTTP client and the existing session. Coordinates came from
the live accessibility tree; Calculator foreground was verified before each tap.
Only the tap HTTP round trip was timed. Setup queries, Windows mouse queueing
and physical/mirrored display latency are excluded.

| Profile | Samples | Mean HTTP time | Median HTTP time | HTTP failures |
| --- | ---: | ---: | ---: | ---: |
| Native `/wda/tap`, original cool-off | 5 | 567.275 ms | 534.457 ms | 0 |
| W3C `/actions`, 50 ms contact, cool-off 0 | 5 | 428.708 ms | 434.050 ms | 0 |

Mean reduction: about 24.4% in this small interleaved test. The user confirmed
the physical iPhone entered the additional digits. The original WDA setting was
restored after testing. This validates that protocol combination; the new Rust
EXE's complete mouse/UI integration requires a separate test.

Phone: iPhone15,4, iOS 27.0; Windows build 26100. Running WDA reported 16.12.7;
source inspection used the setup's upstream tag v16.12.8. These version strings
are not silently equated. Live readback showed idle timeout already 0 and
animation cool-off 2. No gain is attributed to disabling an already-zero idle
timeout. Raw samples: `work/wda-setup/wda-tap-ab.json`. An incorrect initial
PowerShell grouping summary was recomputed from those samples without more taps.

## Kept

- Apply and verify zero idle/animation waits once per session. If the runner
  rejects or does not confirm tuning, retain native tap compatibility.
- Use one complete W3C move/down/50 ms/up request for a warmed tap. Geometry
  stays cached; no per-tap query or parallel input requests are introduced.
- Never replay an ambiguous timeout. Definite expired-session recovery checks
  geometry and tap strategy before a safe replay.
- Add gesture duration to its HTTP timeout, bounded by the existing five-second
  gesture limit. A real five-second swipe must not hit the ordinary four-second
  timeout while still executing.
- Expose the actual tap transport and confirmed setup policy in diagnostics.

Upstream runs a complete synthesized gesture and then its configurable animation
wait: [touch handler](https://github.com/appium/WebDriverAgent/blob/v16.12.8/WebDriverAgentLib/Categories/XCUIApplication%2BFBTouchAction.m),
[settings handler](https://github.com/appium/WebDriverAgent/blob/v16.12.8/WebDriverAgentLib/Utilities/FBSettingsHandler.m).
Removing cool-off permits subsequent input during animation; it does not change
iPhone system animations or auto-accept alerts.

## Reverted: sampled mouse curves

An experimental bounded recorder retained up to 64 mouse points. Before keeping
it, used separate user authorization for two upward Settings swipes, each 300 ms
over the same relative distance with cool-off 0:

| Points | HTTP time, including gesture duration | HTTP result |
| ---: | ---: | --- |
| 2 | 1017.796 ms | Success |
| 8 | 1780.087 ms | Success |

Extra points added about 762 ms in this two-trial check. This is not a statistical
or physical smoothness result, but did not justify adding that cost to every
drag. Removed the recorder, input variant, mousemove handling and curve serializer.
The simple swipe remains. Raw results and the discarded experiment are retained
locally under `work/wda-setup`, outside production source.

WDA still receives and plays a complete gesture after mouse-up. This does not
provide streaming touch or instant hand-following drags. Bluetooth Mouse remains
the verified option for responsive continuous pointer movement. No zero-latency
or fastest-possible guarantee is made.

Rotation/reconnect, object dragging, typing and sustained behavior still need
coverage on the exact integrated EXE. Clean-machine release gates remain open.

## Integrated candidate

`dist/wda-fast-input-20260913/iMirror.exe`, SHA-256
`c577805db7cd72ac01a52ab2d7e83edd9f450f9c342941c1957fbe32baf54db0`.
Fmt, strict Clippy, 73 Rust tests and release build passed. The app launched with
system-only PATH, became WDA Ready and reported W3C tap mode. Independent live
settings readback confirmed both waits are 0. All helper/DLL hashes match the
preceding candidate. The user tested this exact candidate and replied:
“Click nhanh hơn, vuốt nhận đều” (clicks feel faster and swipes register consistently).
Record integrated clicks and simple list swipes as PASS on this physical setup.
This is not a claim about object dragging, long presses or all input scenarios.
