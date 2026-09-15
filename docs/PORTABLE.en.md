# iMirror — getting started on Windows

[Tiếng Việt](PORTABLE.vi.md) · [WDA setup](WDA_SETUP.en.md)

## Download and launch

Download **iMirror-v0.1.0-windows-x64.zip** from the
[project releases](https://github.com/phh235/iphone-mirror-windows/releases).
Use **Extract All** to a stable folder such as `Documents\iMirror`. Open
`START-HERE.html` for offline instructions, or `iMirror.exe` to launch.
Keep the entire folder. Do not run inside the ZIP or copy just the EXE.

USB DLLs, Microsoft runtime files, AirPlay, WDA helpers and notices are included.
Rust, Git, Python, Node, Visual Studio and FFmpeg CLI are not runtime requirements.
This is an **unsigned engineering preview**, with clean-Windows validation still
pending. Verify the official repository and release SHA256 if Windows blocks it;
do not disable antivirus or download random DLLs. The ZIP creates no installer
entry or Start Menu shortcut. You may create a Desktop shortcut to its EXE.

## Choose a mode

- **USB + Bluetooth Mouse:** start here. Video over USB, relative AssistiveTouch
  pointer over Bluetooth. No Apple ID sign-in, Developer Mode, jailbreak or
  phone companion app is required for this combination.
- **Wireless + Bluetooth Mouse:** network video with Bluetooth pointer control.
- **Wireless + USB WDA:** position-based clicks through the live picture, with
  separate advanced phone setup. See [WDA instructions](WDA_SETUP.en.md).
  Swipes execute after mouse release; continuous finger tracking is not implemented.

Audio is disabled. Actual resolution/FPS depend on the device, protocol and host;
60 FPS is not guaranteed.

## USB video

1. Use Windows 11 x64, or Windows 10 22H2 where required APIs/drivers are available.
   Connect a data-capable cable, unlock the iPhone and accept Trust if prompted.
2. If Windows does not recognize it, obtain Apple Devices through
   [Apple's official download instructions](https://support.apple.com/en-us/118290).
   Apple drivers are not bundled. Recognition in Apple Devices alone does not
   guarantee capture compatibility. Do not replace the parent USB driver with Zadig.
3. In **Settings → Connection → USB**, choose the phone, Refresh if needed, close
   Settings and click Connect. Automatic can also connect a detected device.
4. Confirm the picture updates when you use the physical phone. Fit sizes the
   window proportionally; fullscreen may correctly have black bars.

The phone may disappear from This PC during the capture-mode switch. Closing
iMirror hides its window promptly, but USB restoration may continue briefly.
Wait for its process to exit before reopening or removing files.

## Bluetooth pairing and control

1. Enable Windows Bluetooth. In iMirror **Settings → Control**, choose
   **Automatic** or **Bluetooth Mouse**, then enable Control.
2. Wait for pairing instructions. The adapter must support BLE peripheral role;
   ordinary headset support is insufficient. Video remains usable without it.
3. On iPhone, enable **Settings → Accessibility → Touch → AssistiveTouch**.
   Open **Devices → Bluetooth Devices** and select the advertised PC name.
   The developer's PC name is not the name every user should search for.
4. Complete prompts on both devices, then wait for iMirror's **Ready** status.
   Bluetooth Connected does not prove input-report subscription or Control Ready.
5. Click inside the video to capture input. Move the pointer, click, hold left
   button and drag, or scroll vertically with the wheel. Type into a focused
   text field on the phone.
6. Press **Ctrl+Alt+Q** to immediately release local input. Esc is another release
   action. Run one iMirror instance to avoid conflicts over the shortcut.

See [Apple's pointer-device setup](https://support.apple.com/en-us/111775).
Bluetooth Mouse is relative: clicks act at the iPhone pointer, not guaranteed
absolute Windows coordinates. Adjust app sensitivity and iPhone tracking speed.
Zoom is unnecessary. Unexpected panning/dragging warrants releasing capture and
checking Drag Lock, Dwell and Zoom if enabled.

If Control is not Ready, check Bluetooth and AssistiveTouch, then open
**Settings → Advanced → Diagnostics**. If an old pairing is stale, Forget the PC
on iPhone and Remove that iPhone in Windows, then pair again through AssistiveTouch.
Do not remove unrelated devices. No subscriber means control cannot send input.

## Wireless video

Use the same local network. Choose **Settings → Connection → Wireless**, enter
a receiver name, close Settings and Connect. On iPhone select **Control Center →
Screen Mirroring → receiver name**. Allow the intended private network if Windows
asks. Do not disable the whole firewall. Guest networks/client isolation can
prevent discovery. Retain the full AirPlay directory.

If video freezes, Disconnect/Connect and select the receiver again. Higher
quality increases bandwidth; requested FPS is not a source-FPS measurement.
Use Wireless video while USB is reserved for WDA on the tested setup.

## Language, updates and support

Switch English/Tiếng Việt under **Settings → General → Language**. Hover icons
for tooltips; narrow windows move some actions to the overflow menu.

To update, close iMirror, wait for helpers to exit and extract the next ZIP into
a new folder. Do not mix DLLs between versions. Settings remain in
`%LOCALAPPDATA%\iMirror`. To remove the portable app, close it and delete the
extracted folder; separately remove settings only when intentionally resetting
them and after backing up necessary private WDA setup.

Report problems using **Settings → Advanced → Copy Diagnostics**, with version,
connection mode and reproduction steps. Review before posting. Never upload
Apple IDs/passwords, 2FA, UDIDs, pairing records, signing files or private screen
contents. WDA setup belongs to each user's own device; do not share it in the ZIP.
