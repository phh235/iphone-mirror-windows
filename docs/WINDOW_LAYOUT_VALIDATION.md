# Language and phone-shaped window validation

Build: `dist/ui-language-aspect-20260912/iMirror.exe`, SHA-256
`4b81337c06222cfca856d07198bcfa9e8096dd7f2a62f1a233061317e82602f2`.

## Root cause and implementation

The old main window started at a fixed 620×900 effective pixels and did not adapt
its width when the first valid source format arrived. The renderer correctly
letterboxed a tall phone inside that wider video child.

The normal Fit window now calculates video dimensions first, preserving the
effective source ratio including user rotation. It budgets about 88% of the
current monitor work-area height and a 16-dp safety margin, evaluates the compact
toolbar's possible heights, then adds actual caption/borders using
`AdjustWindowRectExForDpi`. The preview continues to fill its host at x=0 with no
added horizontal padding. Captured frames, source resolution, decoder and renderer
are unchanged.

WM_SIZING adjusts the proposed outer rectangle directly; it does not repeatedly
resize the HWND. Width-led and height-led resizes account for toolbar/non-client
height. Monitor/DPI changes and restore paths use the current work area. Toolbar
actions move into the native overflow menu rather than forcing a wider video.
At extremely narrow sizes additional actions remain available in that menu.

Fullscreen/maximized windows retain the renderer's aspect preservation and can
have correct letterboxing. Explicit 1:1 and Fill selections are not overwritten;
automatic phone-shaped sizing applies to normal Fit mode.

## Actual iPhone result

The user confirmed live USB video, a naturally fitted portrait window and no
unnecessary side bars. They also confirmed Rotate to landscape, fullscreen exit
back to Fit and immediate English/Vietnamese switching.
They subsequently confirmed manual resizing preserves the ratio and Disconnect
→ Connect refits the phone correctly.

Device: iPhone15,4, iOS 27.0. Windows build 26100. Measured window DPI: 144 (150%).
Source: **1180×2556**, ratio **0.4616588419405321**. Media Foundation reports
**hardware** decoding before/after this UI work.

| Measurement from live-window diagnostics | Before first autosize | After |
| --- | ---: | ---: |
| Video viewport | 908×1228 | 564×1222 |
| Viewport ratio | 0.7394136808 | 0.4615384615 |
| Expected left unused width | 170.5415 px | 0 px |
| Expected right unused width | 170.5415 px | 0 px |
| Toolbar height | 66 px | 66 px |
| Client area | 908×1294 | 564×1288 |
| Outer window size | 930×1350 | 586×1344 |

These before/after values are actual HWND measurements from the same new process
around its first source-triggered autosize, not a comparison of two independently
timed binaries. Expected padding is calculated from geometry; the user separately
confirmed the visible phone result. Integer window dimensions introduce only
rounding differences. Fit remains selected; no crop/stretch workaround is used.

The snapshot contains one 6.8972-ms decode sample, 1,551 received frames and a
current source-FPS sample of zero. It is not a timed FPS/latency benchmark and
does not establish a regression. Its separate WDA connection error does not
invalidate the live video/window result; backend 0 and zero HID reports mean
this snapshot does not validate Bluetooth input.

## Language settings

Settings → General → Language provides English and Tiếng Việt. The language is
persisted as an optional, backward-compatible configuration field, defaulting
to English for old settings. Relabeling does not recreate transports or change
the selected display mode. Names, diagnostic JSON keys and original driver/API
details are preserved; technical diagnostic contents may remain English.

## Software and remaining hardware coverage

Formatting, strict Clippy, 55 Rust tests and release compilation passed for this
build. Geometry tests cover portrait/landscape, all eight sizing edges, five DPI
values and positive/negative monitor coordinates. Configuration tests verify
language persistence without overwriting Fit/1:1/Fill. App-owned layout fixtures
also exercised native window sizing, both languages, Settings, Rotate and
fullscreen; fixture output is explicitly labeled as software-only evidence.

Physical results confirmed so far: portrait, Rotate landscape, fullscreen exit,
Settings access, immediate language switching, manual resizing and reconnect.
Real multi-monitor/DPI changes,
all first-click stress cases and long-run stability are not established by those
checks. See [current release gates](VALIDATION.md).

Local evidence: `work/language-layout/physical-portrait.json` and the related
software fixture outputs, or their verified cleanup archive after cache removal.
