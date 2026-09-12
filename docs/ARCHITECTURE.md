# iMirror architecture

The Rust Win32 application owns windows, input routing, connection policy,
settings, diagnostics and benchmark reporting. There is no web UI, Electron,
Python runtime or Node.js runtime in the installed application.

## USB and rendering

The pinned GPL native core from RayrenSX/iPhoneMirror supplies QuickTime USB,
CoreMedia parsing, Media Foundation H.264/HEVC decoding and D3D11 presentation.
The retained native core supports WASAPI, but iMirror now opens video-only sessions;
the disabled-audio branch does not allocate PCM outputs or start a playback worker. Rust links it statically through a version-checked C ABI.
The WPF frontend and non-OSI iUsbBridge are excluded.

Cargo also builds the required iPhoneMirror.UsbConfigurationSwitch.exe from
source and places it beside iMirror.exe. Omitting this helper prevented capture
in the earlier development/package build. Apple device support and a compatible
USB backend are still prerequisites; no driver installer is invoked implicitly,
and Apple binaries are not redistributed.

Decoder and renderer run on native worker threads. Rust device commands and
input commands use separate bounded queues, and the UI receives a replaceable
single snapshot. Shared D3D11 textures provide the normal GPU path; the upstream
core retains its CPU materialization/software compatibility paths.

Rendering retains the latest frame. It releases the shared decoder texture
after submitting its sampling work, before nonblocking Present. A temporarily
busy compositor schedules a retry of the newest frame. Fit/1:1, rotation and
VSync are native renderer settings. The process requests 1 ms timer resolution
only while a media session exists and balances it on session destruction.

Source counts deduplicate bounded source-timestamp history. USB counts remain
monotonic across native protocol reconnects; this also prevents unsigned FPS
subtraction from underflowing. Successful render submission counts are separate
from source frames and from monitor refreshes. Present acceptance is not a
screen-to-screen latency measurement.

## AirPlay

A rebuilt, pinned UxPlay process provides AirPlay and built-in mDNS. Bonjour is
not required for this path. The bundled GStreamer pipeline forwards encoded
H.264 through RTP on loopback; Rust validates/reassembles it and feeds the same
Media Foundation/D3D11 engine used by USB. No raw-video pipe or re-encoding is
used. UxPlay is started with `-as 0`; the Rust audio socket/PCM worker is removed.
Existing runtime DLL dependencies remain bundled, but no audio decode/playback
pipeline is requested. No FFmpeg command-line program is shipped.

The Rust reassembler retains one bounded access unit (maximum 8 MiB).
The native encoded decoder queue holds at most three pending units and waits
for a new keyframe after loss/overflow. Unsupported or malformed units are
rejected. Wireless HEVC and measured wireless A/V synchronization remain
unvalidated; the active Rust wireless parser is H.264.

AirPlay starts only on an explicit action. Its child uses private plugin/library
paths, a kill-on-close Windows job, a graceful stop event, and a separate actual
disconnect event. Static video inactivity alone no longer resets AirPlay.
Receiver names and 30/60 FPS source requests are configurable through quality
profiles; a request does not guarantee the source's rate.

## Control and configuration

BLE HID uses WinRT and the MIT windows-ble-hid report design, with peripheral
capability checks, encrypted reports, explicit client selection and input
release on failure. Pairing and responsive relative pointer behavior have been
confirmed on the tested iPhone; exact-release reconnect/keyboard/long-run coverage
remains incomplete. BLE is relative pointer control; WDA is an optional precision path.

WDA uses a native Rust HTTP client restricted to loopback, bounded requests and
session recovery. A signed, installed runner and working local tunnel are
external prerequisites. WDA is not required for video.

Settings are stored per user, validated, atomically replaced and backed up.
Newer config versions are rejected without silently replacing them with an older
backup. Quality affects local USB render limits and wireless source requests;
receiver/profile changes restart an active AirPlay receiver.

## Packaging and acceptance

WiX 3.14.1 produces a per-user MSI and native Setup bootstrapper; a portable ZIP
is also generated. Native input packages and corresponding source archives are
hash pinned. Cargo.lock dependencies, native source packages, patch descriptions
and build recipes accompany the source ZIP. Installed LGPL DLLs remain
replaceable. WiX runtime files retain their MS-RL terms.

See VALIDATION.md for actual evidence. A successful build or desktop-host install
does not replace clean-VM, input, latency, reconnect or sustained-device gates.


## Local cleanup

Generated build/tool/source caches are disposable and ignored by Git. The newest
staged app and local validation evidence are retained by scripts/clean.ps1.

## Source map

The app entry point declares one `ui` module at `crates/app/src/ui.rs`. There is
no second inactive UI implementation. UI styling, SVG/window icons, native input
observations, Settings and diagnostic panels remain separate modules.
`i18n.rs` contains the small English/Vietnamese UI catalog; `window_layout.rs`
calculates aspect-matched window geometry. Both stay outside the media/input
backends. Language changes persist through Config without changing video options.

| Crate | Responsibility |
| --- | --- |
| app | Native UI, ControlManager, Raw Input, settings, diagnostics and orchestration |
| platform-windows | Windows/adapter capability helpers and COM lifetime |
| native-core | Versioned Rust FFI to the pinned C++ USB/decode/render engine |
| device | Configuration validation and reconnect state model |
| video-core | Bounded encoded-video parsing/reassembly interfaces |
| video-airplay | Private UxPlay process lifecycle and loopback encoded transport |
| decoder | MediaFoundationSink connecting encoded frames to the native session |
| coordinate-map | Viewport and rotation transforms used by supported controllers |
| input-core | Input events, bounded coalescing/transitions and metrics |
| input-ble | WinRT HID service, subscription state and report notifications |
| input-wda | Optional loopback-only WebDriverAgent HTTP client |

All workspace crates have consumers; the decoder crate is used by video-airplay,
not an unused replacement for the native decoder. Upstream files remain pinned
and intact even when a particular platform feature is disabled in iMirror.
