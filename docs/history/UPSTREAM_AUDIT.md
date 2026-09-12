> Historical snapshot; see [current validation](../VALIDATION.md) and the [history index](README.md). This record is not current setup guidance.

# Upstream audit — 2026-09-11

Exact source revisions and dates are in upstream-lock.json. Repositories were
cloned and source/build/license files inspected. "Requested FPS" and upstream
reports below are not local hardware results. No physical-phone benchmark has
run. Release activity is an upstream observation, not a stability guarantee.

| Project | License / language | Activity and support | Media, control, dependencies | Decision |
|---|---|---|---|---|
| FDH2/UxPlay | GPLv3; C/C++ | Active through 2026-09-10; native Windows builds through MinGW; current iOS mirroring compatibility depends on protocol | AirPlay H.264/HEVC, GStreamer decode including D3D11; requests 60 but actual source rate unverified; OpenSSL/libplist/GStreamer; no jailbreak | Selected wireless protocol; pin/build and runtime packaging still required |
| danielpaulus/quicktime_video_hack | MIT; Go/C | Head 2023-05-04 README; historical 0.6; README explicitly gives up Windows | QuickTime USB H.264 and PCM, raw recording or GStreamer; no input; libusb; no jailbreak; Windows/FPS not validated | Protocol fixtures/reference only; Windows port is not a release backend |
| danielpaulus/go-ios | MIT; Go | Active 2026-09-04; Windows, Linux, macOS; iOS 17+ tunnels | Native device discovery, signing/tooling, WDA launcher. New encoded RTP displayservice specifically says iOS 27+, older returns 9021; no local max FPS proof. Wintun or supported userspace tunnel; signed WDA/DDI needed for developer services | Optional WDA lifecycle tooling; new video cannot cover iOS 17+ |
| appium/WebDriverAgent | BSD 3-clause; Objective-C | Maintained, 16.12.7 release commit 2026-09-10 | XCTest actions, tap/swipe/keys/buttons. Screen capture/MJPEG is not our media path. Runs on iPhone, not Windows. Xcode/signing/developer mode needed for deployment; no jailbreak | Optional precision control through native Rust HTTP, no Appium server/runtime |
| abhishek-raj/windows-ble-hid | MIT; C# | Active input-disconnect change 2026-09-03; Windows 10 2004+ | HOGP mouse/keyboard, WinRT peripheral role; source reports iPhone input with AssistiveTouch, restart reconnect untested. .NET 8 SDK build; self-contained releases need no installed runtime; no video/no jailbreak | HID service/report reference; native Rust WinRT implementation planned |
| lautarovculic/ioscpy | MIT; Rust/Objective-C | 2026-07-14 README; Windows source support; no upstream Windows binary in README | USB H.264, device-side input tweak; libimobiledevice helpers; requires jailbreak; any reported source FPS not applicable to stock iOS | Rejected for jailbreak requirement |
| 0xbartita/ioscpy-windows | MIT; Rust/Objective-C | Windows fork updated 2026-09-03, packaged Windows workflow | Same jailbreak-only device component, Windows rendering/control improvements and native helper dependencies | Rejected for jailbreak requirement |
| gbulog/pcairplay | MIT integration; GPL UxPlay engine; WPF/C#/scripts | 2026-07-24; Windows and Arch; README reports a real session 2026-07-20 | AirPlay via UxPlay, optional BLE, Windows Bonjour/runtime packaging. No independently measured maximum FPS or latency | Deployment reference only; GPL engine remains GPL despite MIT integration |
| libimobiledevice/libimobiledevice | LGPL-2.1 library; GPL-2 tools; C | Active 2026-06-10; Windows/Linux/macOS; compatibility varies by service/iOS | Device management/usbmux/lockdown; not a continuous encoded screen transport or general stock-iOS input API; libplist/libusbmuxd/crypto/glue dependencies | Reference for Apple transport; no need to ship all tools |
| RayrenSX/iPhoneMirror | GPL-3.0-only core; C++ (WPF frontend excluded) | 2026-09-07; Windows x64, current public-preview lineage; v1.8.3 README | QuickTime/CoreMedia H.264/HEVC; MF/D3D11/WASAPI; libusb0 filter plus Apple driver; no jailbreak; local FPS caps explicitly are not source negotiation. Its optional iUsbBridge is non-OSI and excluded | Reuse native core only, pin exact source, preserve GPL and patch provenance |
| leapbtw/uxplay-windows | GPLv3/UxPlay plus dependency licenses; C/C++/build scripts | Active 2026-09-07; x64 and newer ARM64 build workflows | Native UxPlay packaging, GStreamer/Bonjour support; newest packaging also contains optional Python beacon components | Build reference; exclude Python beacon/runtime from iMirror |

## Decoder decision

Media Foundation was selected because the reused Windows implementation already
has H.264/HEVC negotiation, hardware/software fallback, tested buffer bounds,
and D3D11 shared-texture presentation, rather than because Rust needs a "pure"
Windows stack. Its decoder's actual acceleration report must be exposed.

Direct D3D11 Video decoding requires codec/parser/reference-picture management
currently supplied by MF and offers no demonstrated advantage here. FFmpeg
D3D11VA is a viable future compatibility fallback, but adding its runtime without
a demonstrated MF failure would not improve current evidence. GStreamer D3D11
is mature and relevant to UxPlay; a raw I420 pipe integration causes CPU copies
and must be documented/measured before release. No comparative hardware benchmark
has run yet, so none of these choices is described as universally fastest.

## Distribution obligations

iMirror is GPL-3.0-only. Include GPL corresponding source, our changes and build
instructions, all upstream notices, and source/provenance for redistributed
native libraries. Dynamic LGPL components must remain replaceable. Do not
claim that IPC removes GPL obligations. No Apple proprietary binaries, WDA
signing identities, or non-OSI iUsbBridge binaries are included.

Sources: each repository's README, LICENSE/COPYING, build definitions and source
at the SHA in upstream-lock.json. The full license inventory for the eventual
runtime (including Cargo transitive dependencies) remains a release gate.
