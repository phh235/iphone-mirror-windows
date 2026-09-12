# Compact native UI and Fluent SVG validation (2026-09-12)

Scope: native application chrome and Settings only. No changes to the media
engine, USB/QuickTime/AirPlay, decoder, renderer, BLE protocol, Raw Input
algorithm, or ControlManager. The 198 frozen media hashes match their baseline.

## Implementation

- The bottom action bar is removed, including its reserved viewport space.
  Connect/Disconnect, Rotate, Control, Fullscreen and Settings are right aligned
  in the top bar. At narrow widths, Connect/Rotate move into a native overflow
  menu; supported WDA Home remains accessible there.
- Native owner-drawn BUTTON HWNDs retain their command, accessibility and input
  behavior. Hit area: 34 effective pixels. Icon: 20 effective pixels. Gap: 4;
  corner radius: 6. All scale with DPI; hover/pressed/focus do not change geometry.
- Native Win32 tooltips retain their text buffers and use the system hover delay.
  Connection and capture state update the tooltip without recreating controls.
- Official Microsoft Fluent System Icons Regular SVGs replace the hand-drawn
  GDI icon primitives. Eight required assets total 10,963 bytes. See
  [source pin and MIT attribution](../assets/fluent/README.md).
- Windows `ID2D1DeviceContext5::CreateSvgDocument` parses embedded SVG bytes once.
  `ID2D1SvgPathData::CreatePathGeometry` retains eight device-independent paths
  (361 segments); a cached software `ID2D1DCRenderTarget` and solid brush draw
  them into existing Win32 paint DCs. A temporary WARP device enables the SVG
  parser and is released after extraction. No video device/context is used.
- Foreground paint comes from the current native theme, including system colors
  in high contrast. Original SVG path coordinates are unchanged. Control uses
  the same Regular SVG in every state. No icon font, PNG, framework or external
  runtime is added. Device-loss recovery recreates only the icon DC target.
- Settings default to 840 x 580 effective pixels, with a 160-pixel sidebar and
  38-pixel navigation rows. The content pane scrolls independently; navigation
  and Done remain pinned. Wireless fields appear only in Wireless mode.
- Sensitivity retains its native trackbar input/keyboard behavior. A single
  custom paint pass draws its track and thumb without the stock white background.

## First-click and focus evidence

The earlier fix in `7579dae` remains: delayed preview focus cleanup can release
only capture owned by the preview HWND, never capture held by a native button.
BN_CLICKED is validated against the sending child HWND; the command is dispatched
once after the native callback unwinds. This is not a retry or synthetic click.

The hidden Win32 regression test exercises 100 capture-ownership cycles. It does
not prove 100 physical clicks. The icon conversion preserves native button
semantics and keyboard routing. Solid inset focus outlines replace DrawFocusRect;
mouse modality hides focus cues, while keyboard modality restores them.

Physical first-click stress, inactive-window activation, tooltip UX, Tab/Space/
Enter and Ctrl+Alt+Q-followed-by-click acceptance on this exact build remain
UNTESTED. The desktop automation helper was unavailable in this environment.

## Software checks and measurements

- `cargo fmt --check`: PASS.
- `cargo clippy --all-targets --all-features -- -D warnings`: PASS.
- `cargo test`: PASS, 52 tests.
- `cargo build --release`: PASS.
- 50 app-owned UI captures/navigation checks: PASS. Main plus four Settings
  pages, dark/light, each at 96/120/144/168/192 test DPI. This exercises real
  HWND layout/painting with a test DPI override, not physical monitor changes.
- Two additional small-window navigation checks at 96/192 DPI: PASS; the
  content remains reachable and Done remains pinned after scrolling.
- Three independent `--ui-icon-benchmark` processes: PASS. Each verifies all
  eight SVGs at five sizes and three foreground/background palettes (120 cases),
  then measures 1,000 warm paints per DPI, including Direct2D EndDraw.
- SVG parse/cache initialization: 35.29–39.31 ms across the three processes.
  Retained private-memory increase after initialization: 1.70–1.76 MiB, including
  native library initialization. Private bytes after paint: 5.52–5.60 MiB total
  for the standalone benchmark. Native COM geometry allocation size is opaque;
  these process deltas must not be described as exact path-buffer bytes.
- Icon-only warm paint average: 0.285–0.312 ms; p95: 0.364–0.419 ms across fixed
  DPI cases. These are CPU/DC measurements, not full-window repaint or phone
  latency. No continuous icon animation or repaint timer was introduced.
- Actual Windows high-contrast mode switch: UNTESTED. The render test uses current
  system foreground/background colors; it does not change the user's OS theme.
- Screenshots are software UI checks with no live phone session. Connected and
  captured screenshots, physical mirror and input regression remain pending.

## Matched idle comparison

Two 45-second runs per build, five seconds of warm-up, same isolated UTF-8
settings (manual USB, Control OFF). Native logs were checked for capture and the
process-specific control snapshot confirmed backend 0/capture false. No builds
or other UI benchmarks ran during these measurements. CPU is normalized over
logical processors. RAM values average the final sample from each short run.

| Build | Working set MiB | Private MiB | CPU % | First HWND ms (range) |
| --- | ---: | ---: | ---: | ---: |
| Original Phase 5 r2 | 25.48 | 5.13 | 0.0577 | 63.0–102.0 |
| Compact UI, hand-drawn icons | 25.91 | 5.07 | 0.0561 | 65.0–358.7 |
| Compact UI, Fluent SVG | 30.40 | 7.05 | 0.0708 | 79.6–96.7 |

Native SVG adds about 4.48 MiB working set and 1.98 MiB private memory relative
to the compact hand-drawn version. The CPU difference is 0.0148 percentage
points in this small sample. First HWND timing is process metadata, not a
first-painted-frame measurement. Two short warm runs do not establish statistical
significance, cold-start behavior, or long-term memory stability. Keep the SVG
change for consistent official assets and DPI-correct vector rendering; it is
not a memory optimization. EXE size changes from 3,146,240 to 3,191,808 bytes.

## Exact engineering artifact

`D:\airplay-iphone\dist\ui-fluent-svg-20260912\iMirror.exe`

SHA-256: `550761221e5393458f321c53184c6b0f7401cbc663855e5c466c47840c0bf3aa`.
The complete folder includes USB helpers, official Microsoft application-local
CRT, the unchanged private AirPlay runtime, licenses and Fluent SVG attribution.
This is an unsigned local test build, not a published installer release.

Machine-local evidence is under `work/ui-polish`: `svg-ui-results.json`,
`svg-final-{1,2,3}.json`, the corresponding BMP captures and `idle-final/`.

The first idle attempt was invalid: PowerShell 5 wrote BOM-prefixed test JSON.
Only the isolated test profile was affected; user settings were untouched.
The script now writes explicit UTF-8 without BOM. A subsequent run was rejected
after actual USB capture appeared in its log. Neither run is a valid idle baseline.
