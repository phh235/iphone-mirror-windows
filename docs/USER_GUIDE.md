# Using iMirror

[Tiếng Việt](../HUONG_DAN.md) · [README](../README.md)

This guide matches the compact native UI. See [validation status](VALIDATION.md)
for what has actually been tested. USB video and Bluetooth relative pointer
control have worked on a real iPhone; this is still an engineering preview.

## Before you start

Use Windows 11 x64, or Windows 10 22H2 where the required APIs and drivers work.
For USB, use a data-capable cable and an unlocked iPhone. Install compatible
Apple device software from the [official Apple Devices download](https://support.apple.com/en-us/118290)
if it is missing. Apple Devices is a prerequisite on some setups, not a guarantee
of QuickTime USB compatibility. Do not replace the iPhone's parent USB driver
with WinUSB as a troubleshooting shortcut.

For a validated installer release, install the Setup EXE and launch from Start
Menu. If you were given a local engineering folder, extract it completely and
keep its DLLs, USB helper, licenses and `AirPlay` folder beside `iMirror.exe`.
The installed app does not need Rust, Python, Node.js or developer tools.
No current clean-machine installer validation is claimed.

## USB mirroring

1. Connect the iPhone, unlock it and accept **Trust This Computer** if prompted.
2. Open iMirror. **Automatic** attempts a detected USB phone.
3. If needed, choose **Settings → Connection → USB**, select the phone under
   **Device**, use **Refresh**, then close Settings and click **Connect**.
4. Check that the status says **Connected** and the picture responds to actions
   on the physical phone.

USB mirroring and Bluetooth Mouse do not require jailbreak, an iPhone companion
app, Apple ID sign-in or Developer Mode. Bluetooth control can stay off while
you mirror. Audio playback is disabled in this product.

Before video arrives, the normal window is a short connection panel. It expands
to the phone's proportions when video starts and returns to the compact panel
after an explicit disconnect finishes.

During USB mirroring, the iPhone can temporarily disappear from **This PC** as
USB switches from photo browsing to video capture. Closing the main window
releases local control and hides the UI first; USB restoration may continue in
the background. Let it finish before reopening the app or removing its files.

## Toolbar

Change language under **Settings → General → Language**: **English** or
**Tiếng Việt**. The app relabels its UI immediately and saves your choice. Device
names, API names, diagnostic data and original driver/OS errors remain unchanged.

In normal **Fit** mode, the window sizes its video viewport to the source aspect
ratio, accounting separately for toolbar, caption/borders and DPI. Manual edge
resizing stays proportional. Rotation, reconnect and monitor changes recalculate
the normal window without changing captured frames or source resolution.
Fullscreen can correctly show black bars on a differently shaped monitor.
Explicit **1:1** and **Fill** selections are not overwritten by aspect matching.

Hover an icon for its native tooltip. The right-hand actions are:

| Action | Purpose |
| --- | --- |
| Connect / Disconnect | One action that follows connection state |
| Rotate | Rotate the displayed video |
| Control iPhone | Enable or disable phone control |
| Fullscreen / Exit fullscreen | Toggle fullscreen |
| Settings | Open configuration |

In a narrow window, Connect and Rotate move into **…**. Home appears in that
menu only when the selected advanced backend actually supports it.

## Bluetooth Mouse setup

1. Turn on Windows Bluetooth. Enable **Control** in the toolbar or
   **Settings → Control → Enable Control**. Leave the mode as **Automatic** or
   **Bluetooth Mouse**.
2. Wait for iMirror's pairing guidance. An unsupported peripheral-role adapter
   cannot provide this control backend; mirroring can still be used.
3. On iPhone, enable **Settings → Accessibility → Touch → AssistiveTouch**.
   Under **Devices → Bluetooth Devices**, choose the advertised PC and complete
   pairing requests on both devices.
4. Wait until iMirror's Control status says **Ready**, then click the video to
   capture input. Bluetooth **Connected** alone does not establish Control Ready.

See [Apple's pointer-device instructions](https://support.apple.com/en-us/111775).
Zoom is not required for Bluetooth Mouse.

## Everyday control

- Movement controls the iPhone's AssistiveTouch pointer. Left-click acts at that
  pointer; hold and move to drag/swipe; the wheel scrolls vertically.
- Type into a focused iPhone text field when the Bluetooth keyboard connection
  is ready. Keyboard layouts, IMEs and special keys need broader physical testing.
- **Ctrl+Alt+Q** immediately returns local control to Windows. **Esc** is an
  additional capture-release key. Focus loss also releases capture.
- **Ctrl+Shift+F** toggles fullscreen while the video has keyboard focus.
- Control's background indicates enabled/captured state; the icon never changes size.

This is relative mouse control, not absolute touch at the Windows cursor's
mirror coordinates. Watch the AssistiveTouch pointer to see where a click acts.
Set **Settings → Control → Pointer sensitivity** between 5–200%; new settings
default to 100%, and saved preferences persist. iOS acceleration can differ
from Windows. Sensitivity changes movement distance, not Bluetooth latency.

## Settings

| Page | Options |
| --- | --- |
| General | English / Tiếng Việt; applied immediately and saved |
| Connection | Automatic, USB, Wireless; device selection and Refresh. Receiver name appears in Wireless mode |
| Control | Enable Control, input mode, sensitivity, release shortcut and readiness |
| Display | Fit, 1:1, Fill, Synchronized display |
| Advanced | Diagnostics, Copy Diagnostics and optional advanced features |

Fit shows the entire image. 1:1 uses one source pixel per display pixel. Fill
can crop the image edges. Changing connection mode disconnects the active
session. Changes save automatically; Done closes Settings.

## Wireless

Put both devices on the same local network. Select **Settings → Connection →
Wireless**, set the receiver name, close Settings and click **Connect**. On
iPhone, open **Control Center → Screen Mirroring** and choose that name.
If prompted by Windows, allow only the intended private network. Do not disable
the firewall globally. Guest-network isolation can prevent discovery.

Wireless needs the supplied private runtime. Bluetooth carries control, not
video. A requested frame rate is not proof of actual wireless source FPS.
Live video is confirmed on one iPhone; long-run and network-loss/reconnect
coverage remain pending. See [the tested build and limits](WIRELESS_BLACK_SCREEN_DIAGNOSIS.md).

## Troubleshooting

| Problem | Check |
| --- | --- |
| No USB phone | Unlock/Trust, data cable/port, Refresh and whether Apple Devices recognizes it |
| Device listed but no video | Select it and Connect; export diagnostics before attempting driver changes |
| No PING / libusb0 -116 | Close iMirror, let USB cleanup finish, then replug with the phone unlocked. If this persists, restarting the iPhone recovered the tested case; this is not a general compatibility guarantee |
| Bluetooth connected, Control not ready | AssistiveTouch, Windows Bluetooth and app status; if necessary Forget/Remove the pairing on both devices, then pair from AssistiveTouch |
| Cursor too fast/slow | App sensitivity and iPhone tracking-speed settings |
| Screen moves while merely hovering | Release capture; check iPhone Drag Lock, Dwell and Zoom/Pan settings if enabled |
| Input remains captured | Press Ctrl+Alt+Q; Esc is a secondary release |
| Missing DLL | Restore the complete app folder; do not download arbitrary DLL files |
| Settings malformed | Close the app and preserve a backup of its settings for diagnosis; do not silently overwrite them |

Use **Settings → Advanced → Copy Diagnostics** when reporting a problem. Include
app/Windows/iOS versions, transport and exact reproduction steps. Review the
report before sharing; never attach credentials, pairing files, UDIDs, private
screen contents or raw packet logs to a public issue.

Settings live in `%LOCALAPPDATA%\iMirror` (`settings.json`, `ble-input.json`).
Source FPS differs from monitor refresh rate. Decode/input timings are local
software measurements, not screen-to-screen latency.

## Advanced WDA

For first-time setup, use the [complete Windows WDA guide](WDA_SETUP.en.md),
including Sideloadly, Developer Mode, developer-image registration and a real
click test. For ZIP installation, see the [portable guide](PORTABLE.en.md).

Enable advanced features only if you already have a signed, installed
WebDriverAgent runner and have completed its developer connection setup.
The WDA option then appears in Control settings. Phone-side signing and Developer
Mode requirements apply; deployment uses an appropriate Apple signing identity.
iMirror does not provide signing credentials or bundle a runner. Normal USB and
Bluetooth Mouse operation does not need WDA. One Windows click on live Wireless
video has been confirmed to tap the physical iPhone through WDA. Simple list
swipes are also confirmed; object dragging, long presses and typing remain untested.

For the validated combination, use **Wireless video + USB for WDA**. QuickTime
USB mirroring interrupted the WDA USB tunnel on the tested host. After the
existing signed runner and private connection setup are registered, iMirror
starts and supervises its own helpers. Leave WDA selected and Control enabled;
reopening the app automatically reconnects. Keep the complete application folder,
including `WDA`. Do not run the old manual helpers alongside it.

The user confirmed automatic connection and a physical click after reopening.
Initial signing, trust, Developer Mode and developer-image setup remain required;
iMirror does not renew signing or mount the developer image after a phone reboot.
See [setup, lifecycle and exact validation](WDA_MANAGED_RUNTIME.md).

WDA is a precision automation path, not an instant mouse transport. The tested
geometry-cache build felt slightly faster, but observed software tap dispatch
still took about 0.6 seconds at the median. See [measured limits](WDA_PERFORMANCE_VALIDATION.md).
The later W3C tap candidate reduced average HTTP tap time from about 567 to
429 ms in a small real-device comparison. This is not a zero-latency guarantee;
see [current tuning and candidate status](WDA_LATENCY_TUNING.md). WDA swipes still
play after releasing the mouse; they do not continuously follow a held mouse.
If a managed helper stops, iMirror invalidates control and retries automatically.
Wait for **Advanced control connected**; if it stays unavailable, inspect
**Settings → Advanced → Diagnostics** for the runtime reason. The manual
**Connect WDA** action remains available. The current build prevents two normal
instances from competing for WDA and the release shortcut.

## Uninstall or reset

Uninstall an installed package through **Windows Settings → Apps → Installed
apps → iMirror**. For a local engineering folder, close the app before deleting
that folder. User settings are stored separately; back them up before removing
`%LOCALAPPDATA%\iMirror` if you want a complete reset.
