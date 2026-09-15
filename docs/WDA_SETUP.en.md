# Optional WDA setup on Windows

[Tiếng Việt](WDA_SETUP.vi.md) · [iMirror and Bluetooth](PORTABLE.en.md)

## Requirements and limits

Skip this guide for Bluetooth Mouse. WDA uses a signed iPhone runner and XCTest.
It has noticeable latency; swipes execute after mouse release. Use **Wireless
video + USB for WDA**: QuickTime USB capture interrupted the WDA connection on
the tested host. Do not run separate old WDA/tunnel helpers alongside iMirror.

The ZIP includes native `WDA\ios.exe`, `wda-forwarder.exe` and registration tools.
It excludes Sideloadly, signed runners, Apple images and private pairing material.
A compatible prebuilt runner can be used from Windows; building/modifying the
runner requires Mac/Xcode. A complete fresh-Windows WDA setup has not been validated.

## 1. Obtain the runner and signing tool

Connect/unlock the iPhone, accept USB Trust and verify device detection.
Download Windows x64 Sideloadly from [its official site](https://sideloadly.io/).
Redistribution permission has not been established, so its installer is linked,
not repackaged. Consult the [official FAQ](https://sideloadly.io/faq) for its
iTunes/iCloud requirements; these differ from Apple Devices USB detection.
Do not replace working USB drivers as a guess at resolving signing failures.

Obtain the real-device **iphoneos**, not simulator, runner from
[Appium WebDriverAgent v16.12.8](https://github.com/appium/WebDriverAgent/releases/tag/v16.12.8),
the version used in the tested setup. This is not a compatibility promise for all iOS versions.

If the asset is a ZIP containing `WebDriverAgentRunner-Runner.app`, prepare an IPA:

```text
Payload/
  WebDriverAgentRunner-Runner.app/
    Info.plist
    ... all original runner files, frameworks and plugins ...
```

Extract the complete app into Payload, compress **Payload** into ZIP, then rename
the ZIP extension to `.ipa`. Enable Explorer's file-name extensions first. Do not
include an extra outer directory or remove XCTest plugins. Renaming the upstream
ZIP alone does not add the required Payload structure. An Invalid file error
requires checking the archive, not repeatedly signing the same malformed file.

## 2. Sign and trust

In Sideloadly choose your iPhone and IPA, use your own Apple Account and personally
handle password/2FA prompts there. Do not send credentials to iMirror's author.
After Done, open iPhone **Settings → General → VPN & Device Management**, choose
the matching developer app identity and Trust/Verify if requested.

Sideloadly documents a usual seven-day free-account signing lifetime; renew with
the appropriate identity/bundle. iMirror does not renew signatures. An installed
icon or Done message does not prove XCTest can launch. Nested-plugin signing
errors require correcting runner signing, not changing the mirror connection.

## 3. Enable Developer Mode on the phone

Go to **Settings → Privacy & Security → Developer Mode**, enable it and accept
the restart. Unlock afterwards, confirm enabling and enter the phone passcode
when requested. If the setting is absent, cable connection alone may not expose
it; finish development-signed app installation or developer pairing first.
See [Apple's instructions](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device)
and [Apple's Developer Mode explanation](https://developer.apple.com/videos/play/wwdc2022/110344/).
Do not install unknown configuration profiles to expose the setting. It is not
required for Bluetooth Mouse.

## 4. Supply a compatible Apple developer image

Obtain a developer support image you are entitled to use from
[Apple Developer Downloads](https://developer.apple.com/download/all/) or your
matching Xcode environment. For iOS 17+, go-ios expects the **Restore directory**
with `BuildManifest.plist` and its complete image/trustcache files, not a standalone
DMG filename. A filename/build number alone does not prove compatibility.

Keep this directory in a stable private location outside the disposable app
folder. iMirror does not bundle or automatically download Apple images. If you
cannot obtain a compatible image or extract Restore, WDA setup is incomplete;
Bluetooth Mouse remains usable. Preparing the image may require assistance from
someone with the appropriate Apple/Xcode environment.

## 5. Register your own device

Close every iMirror. Open built-in Windows PowerShell in the extracted app folder:

```powershell
.\WDA\ios.exe list
.\WDA\ios.exe apps --list --udid=YOUR_DEVICE_ID
```

Use the identifier from the first command. Find the installed, signed runner's
bundle identifier; signing can change it, so do not copy a sample ID blindly.
Keep these results private.

Double-click **Configure-WDA.cmd**. Enter the device ID, installed runner bundle
ID and Restore path. This stores private registration under
`%LOCALAPPDATA%\iMirror\wda\setup.json`; it does not sign/install an app, ask for
passwords, change drivers or copy images into the release. Built-in PowerShell
is sufficient. If you already have this device's own developer pairing folder,
the advanced registration command can import it:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\register-wda-runtime.ps1 -DeviceId "YOUR_DEVICE_ID" -RunnerBundleId "YOUR.RUNNER.BUNDLE" -DeveloperImagePath "C:\YOUR_IMAGE\Restore" -PairingSource "C:\YOUR_PRIVATE_PAIRING"
```

Replace all placeholders. PairingSource is optional; never import someone else's
pairing files. Do not upload setup.json, pairing records, `.p12` or provisioning files.

## 6. Start and verify a real click

Keep USB attached and the iPhone unlocked. Open one iMirror. Enable
**Settings → Advanced → Enable advanced features**, then choose
**Control → Advanced automation (WDA)** with Control enabled.
Wait for **Advanced control connected**. The manager checks/mounts the registered
image before tunnel/runner startup; personalization may require access to Apple.

Choose Wireless in Connection settings, Connect and select iMirror in iPhone
Screen Mirroring. Open Calculator at 0 and click 1 once through the live picture.
Only a physical phone response establishes success. A list swipe is hold, drag,
release; continuous finger tracking is not implemented.

After phone restart, unlock it and reopen iMirror. Automatic image remount and
a subsequent click passed on one device with this recovery build. Expired
signing, lost Trust, disabled Developer Mode or an incompatible image after an
iOS update still require their respective setup actions.

## Troubleshooting

- **Not Ready:** inspect Settings → Advanced → Diagnostics for the runtime reason.
- **Image missing:** register Restore and retain that cache across app updates.
- **Signing/trust:** renew/trust the runner and check its actual bundle identifier.
- **Locked/unavailable:** unlock with passcode and check USB/Trust.
- **Port occupied/agent missing:** close old manual helpers; do not open more app
  copies or kill unrelated processes.
- **Connected but no input:** check WDA selection, Control enabled and live video.
  Zero Bluetooth subscribers is expected when only WDA is being used.

Share sanitized diagnostics and reproduction steps. Sideloadly/Apple are separate
providers; iMirror never needs your Apple password.
