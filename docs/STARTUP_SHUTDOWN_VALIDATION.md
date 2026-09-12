# Compact startup and responsive shutdown — 2026-09-12

Candidate: `dist/ui-startup-shutdown-20260912/iMirror.exe`, SHA-256
`6064e68e664f8bf0506e5f93098accff3a03a049b9526ba45b3c7c70324d731e`.
This is a local engineering build, not a validated public installer.

## Changes and limits

The disconnected normal window starts with a 440x230 effective-pixel client
area, capped to the current monitor work area and adjusted for actual DPI and
non-client borders. At the first valid source format, existing Fit/aspect sizing
takes over. Explicit disconnect returns to the compact panel after the worker
reports inactive. Fullscreen and maximized windows retain their existing rules.

Previously, the UI quit its message loop and joined workers before destroying
the visible window. Native USB restoration could keep the window visible for
many seconds. Shutdown now requests both worker groups to stop, releases local
input and hides main/owned windows immediately. It retains HWNDs and pumps
messages until both worker groups finish, then joins and destroys them.

The process can remain alive while USB restoration finishes. No USB timeout,
native capture, decoder, renderer, BLE protocol or input scaling change is part
of this patch. Rust worker/control/raw-input edits affect stop requests and
completion checks. Pending commands are not started after shutdown is requested.

## Software checks

- Formatting, Clippy with warnings denied, 56 Rust tests and release build pass.
- Geometry tests exercise the compact panel at 100%, 125%, 150%, 175% and 200%
  DPI, including a short work area with negative monitor coordinates.
- Three real app-owned idle smoke processes showed their windows and closed via
  WM_CLOSE. Window hiding took 4.36, 7.11 and 4.24 ms; observed worker completion
  took 386.40, 167.25 and 382.36 ms. The second run included Settings.
- An inspected native capture at 150% DPI shows a compact 682x401 outer window.

These are software timestamps starting at WM_CLOSE handling, not physical click
latency. Worker completion is sampled by the existing UI timer. Idle checks do
not validate shutdown of a live USB session.

## Physical investigation

The user initially reported a compact window and fast close, then reported that
USB could not connect. Do not count that initial answer as a completed USB
regression pass. Logs showed device enumeration, USB configuration activation
and interface claim succeeding, but no QuickTime PING and zero decoded frames;
the handshake recovery write returned libusb0 -116. Cable replug did not recover.

All 80 runtime/helper files compared identically with the preceding physically
validated build (`ui-language-aspect-20260912`, SHA-256
`4b81337c06222cfca856d07198bcfa9e8096dd7f2a62f1a233061317e82602f2`). That reference
also reproduced the handshake failure. The same error appears in earlier logs
at 17:21 and 17:24, before this candidate was launched. This narrows the
investigation but does not establish the underlying driver/phone failure cause.

Candidate process 26536 hid its window in 4.86 ms; the control group stopped in
125.81 ms and the media worker in 17,406.57 ms. That attempt's native USB restore
timed out without confirming normal configuration. Fast window hiding must not
be confused with successful USB restoration or instant process termination.

After the user restarted, unlocked and reconnected the iPhone, one Connect in
the reference resumed H.264 video at 18:32:59: 1180x2556, hardware decoding and
real D3D submissions. The user confirmed video appeared. No driver installation,
service restart or native protocol edit was performed to obtain that recovery.

The candidate then started real video at 18:34:33 (process 13996): 1180x2556,
hardware decoder, viewport resized from 660x279 to 564x1222. The user confirmed
live video, correct fit and fast close. Its window hid in 5.58 ms; the control
group stopped in 103.12 ms and media completion was observed at 15,549.68 ms.
All 1,243 input frames were decoded during that short session. This was not a
controlled FPS/performance comparison.

The phone acknowledged both stop messages and both releases, but native normal
USB configuration verification timed out. A later Windows PnP read showed the
Apple nodes OK; that alone did not prove USBMux readiness. The next candidate
process (30648) enumerated zero devices. After the user replugged/unlocked the
phone, video returned automatically without another phone restart. The user
confirmed that recovery. A direct read-only USBMux ListDevices query then
returned one device; no pairing records or identifiers were exported.

Physical compact sizing, Fit and fast window hiding are confirmed for this
candidate. Reopening without cable intervention is NOT a passed regression
gate: USB restoration/re-enumeration remains unreliable on this host. No new
native USB fix is claimed. Long-run stability and clean-machine installation
remain outside this result.

Local evidence: `work/shutdown-validation/idle-results.json`, smoke BMPs,
`usb-investigation.json`, captured native session logs and the app's bounded
`%LOCALAPPDATA%\iMirror\shutdown.json` report. These files are ignored by Git;
bulky captures may subsequently be stored in the verified cleanup archive.
