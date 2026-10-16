<img src="assets/logo.png" width="96" alt="iMirror logo">

# iMirror — iPhone Screen Mirroring for Windows

Open-source iPhone screen mirroring and control for Windows. Mirror your iPhone
to a PC over **USB** or **wireless AirPlay**, then use a **Bluetooth mouse and
keyboard** with AssistiveTouch, or optional **WebDriverAgent (WDA)** control.

Built with Rust, windows-rs and Win32, with Media Foundation decoding and D3D11
video rendering. No Electron, WebView, Node.js or Python runtime in the app.

**Tiếng Việt:** Phản chiếu màn hình iPhone lên máy tính Windows qua USB hoặc
Wi-Fi, điều khiển bằng chuột và bàn phím Bluetooth; WDA là lựa chọn nâng cao.

[Hướng dẫn tiếng Việt](HUONG_DAN.md) · [User guide](docs/USER_GUIDE.md) ·
[Build from source](docs/BUILDING.md) · [Contributing](CONTRIBUTING.md)

**[Download for Windows x64 — portable preview ZIP](https://github.com/phh235/iphone-mirror-windows/releases/download/v0.1.0-preview.1/iMirror-v0.1.0-windows-x64.zip)**
· [Release notes and checksums](https://github.com/phh235/iphone-mirror-windows/releases/tag/v0.1.0-preview.1)

The download is **v0.1.0-preview.1**, an **unsigned engineering preview**.
Clean-Windows installation and broader device compatibility are not yet validated.

## Current status

**Engineering preview, not yet a validated public installer release.** USB video
and Bluetooth relative pointer control have worked on a real iPhone. Wireless
video and one WDA click through the live image are also physically confirmed.
Broader hardware compatibility, sustained stability and clean-machine installation
still require validation. See [test evidence and release gates](docs/VALIDATION.md).

| Area | Current implementation |
| --- | --- |
| USB mirroring | QuickTime-compatible encoded video, native hardware/software decoder selection |
| Control | Bluetooth relative mouse and keyboard with iPhone AssistiveTouch |
| Windows UI | Compact native toolbar, system theme, DPI-aware controls and Fluent SVG icons |
| Language | English / Tiếng Việt in Settings → General; saved and applied immediately |
| Wireless | Live video confirmed on one iPhone; long-run and loss/reconnect coverage pending |
| Advanced control | Wireless + WDA clicks and automatic reconnect after app reopening confirmed; initial signed runner/developer setup required |
| Audio | Disabled; this app focuses on mirroring and control |

Targets: Windows 11 x64 and Windows 10 22H2 x64 where APIs/drivers permit; iOS 17+
is the compatibility goal, not a guarantee. Actual FPS, resolution and latency
depend on the phone, protocol and PC. USB and Bluetooth control are separate
connections; USB mirroring does not require Bluetooth control or WDA.

In normal Fit mode, the window follows the connected video's aspect ratio and
stays proportional when resized. Fullscreen preserves the image and may correctly
show black bars. Explicit 1:1 and Fill selections are retained.

## Start using iMirror

Download the **Windows x64 portable ZIP** linked above, or choose it under
[GitHub Releases](https://github.com/phh235/iphone-mirror-windows/releases).
Extract everything and open `START-HERE.html` for Vietnamese/English instructions,
or `iMirror.exe` to run. The corresponding-source ZIP is for developers,
not the application download.

[Portable guide — Tiếng Việt](docs/PORTABLE.vi.md) ·
[Portable guide — English](docs/PORTABLE.en.md) ·
[WDA setup — Tiếng Việt](docs/WDA_SETUP.vi.md) · [WDA setup — English](docs/WDA_SETUP.en.md)

**Set up with an agent:** copy the [Tiếng Việt](docs/AGENT_SETUP.vi.md) or
[English](docs/AGENT_SETUP.en.md) prompts for app setup, Bluetooth, Wireless,
optional WDA, signing renewal and troubleshooting.

For a supplied engineering app folder, keep **all** files beside `iMirror.exe`,
including the USB helpers, DLLs, licenses and `AirPlay`/`WDA` directories. Do not copy the
EXE alone. A fresh Git checkout contains source, not a prebuilt application.

1. Connect the unlocked iPhone by USB and accept **Trust This Computer**.
2. Open iMirror. **Automatic** tries a detected USB phone; otherwise use the
   **Connect** icon. Choose a device in **Settings → Connection** if necessary.
3. For mouse control, enable **Control**, enable iPhone **AssistiveTouch**, and
   pair the PC when iMirror says Bluetooth control is ready for pairing.
4. Click inside the mirrored image to capture input. **Ctrl+Alt+Q** releases it
   immediately; **Esc** also releases captured input.

Detailed setup, icon meanings, sensitivity, wireless and troubleshooting:
[English](docs/USER_GUIDE.md) / [Tiếng Việt](HUONG_DAN.md).

## iPhone mirroring and control FAQ

### Can I mirror an iPhone to Windows using a USB cable?

Yes, on compatible devices and drivers. Connect an unlocked iPhone, accept
**Trust This Computer**, and select USB or Automatic in iMirror. Compatible
Apple device software is required for USB connection. Follow the
[USB setup and troubleshooting guide](docs/USER_GUIDE.md#usb-mirroring).

### Can I mirror my iPhone wirelessly over Wi-Fi?

iMirror includes an AirPlay receiver for Windows. Put the iPhone and PC on the
same local network, select Wireless in iMirror, then choose its receiver in
**Control Center → Screen Mirroring**. See [wireless setup](docs/USER_GUIDE.md#wireless)
for network requirements and the current validation limits.

### Can I control my iPhone with a Windows mouse and keyboard?

Bluetooth Mouse uses iPhone AssistiveTouch for relative pointer movement,
clicking, dragging and scrolling, with keyboard input when its connection is
ready. It needs a compatible Windows Bluetooth adapter with BLE peripheral-role
support. Video and control use separate connections; AirPlay does not carry
mouse commands. Follow the [Bluetooth control guide](docs/USER_GUIDE.md#bluetooth-mouse-setup).

### Do I need a Mac or a signed iPhone app?

USB mirroring and Bluetooth Mouse do not require a Mac, jailbreak, Apple ID
sign-in or a signed companion app. Optional WDA control requires Developer Mode
and a separately signed/trusted runner on the iPhone. A compatible prebuilt
runner can be signed from Windows; building or modifying the runner requires
Mac/Xcode. See [WDA setup](docs/WDA_SETUP.en.md).

### Does iMirror guarantee 60 FPS or realtime touch control?

No. Actual source and display FPS depend on the iPhone, connection and PC.
WDA swipes currently execute after mouse release; they are not continuous
realtime touch. Audio playback is disabled. See [validation and release gates](docs/VALIDATION.md)
for tested behavior and unresolved limitations.

## Development

Rust MSVC and the Visual Studio C++ Build Tools/Windows SDK are needed to build
the native app. Additional tools are needed only for packaging the wireless
runtime and installers. End users do not need these tools.

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

[Build instructions](docs/BUILDING.md) explain prerequisites, output files,
packaging, tests and cleanup. [Architecture](docs/ARCHITECTURE.md) describes the
module boundaries. Historical experiments are indexed under
[docs/history](docs/history/README.md); they are not current setup instructions.

To keep the checkout small after building, use PowerShell 7:

```powershell
pwsh -File scripts/clean.ps1 -Preview
pwsh -File scripts/clean.ps1
```

Cleanup preserves the newest staged app, source, Git history and rollback refs.
It archives and verifies bulky test evidence before removing it. Build caches,
local diagnostics and packaged binaries are ignored by Git and regenerate when
needed. See [cleanup details](docs/BUILDING.md#cleanup).

## License and attribution

iMirror is licensed under [GPL-3.0-only](LICENSE). The native media components,
Bluetooth HID reference, Fluent icons and other dependencies retain their own
licenses and notices. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) and
[distribution requirements](docs/LICENSING.md). A process boundary does not by
itself remove GPL obligations.

iMirror is an independent project, not affiliated with, endorsed by, or sponsored
by Apple Inc. or Microsoft Corporation. Product names and trademarks, including
iPhone, iOS, AirPlay and Windows, belong to their respective owners.
No Apple proprietary binaries or signing credentials are distributed here.
