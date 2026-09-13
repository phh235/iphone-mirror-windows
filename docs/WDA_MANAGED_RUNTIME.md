# Managed WDA runtime — September 13, 2026

This optional advanced backend starts the existing signed WDA runner and its
Windows connection when Control is enabled with WDA selected. It does not sign
or install a phone app. Bluetooth Mouse remains the normal control backend.

## Using the validated setup

Keep the trusted iPhone connected by USB and unlocked. Use **Wireless** for
video, enable Advanced features, select WDA in Control, and leave Control enabled.
Once this preference and the private setup registration exist, reopening iMirror
starts the runtime automatically. Wait for **Advanced control connected** before
clicking the live picture. Do not also run the old manual WDA helpers.

Keep the complete application folder, including its `WDA` subfolder; copying
only `iMirror.exe` omits required helpers. The developer does not need to run a
terminal for each app launch after the initial setup has been registered.

Use **Wireless video + USB for WDA** on the tested host. Starting QuickTime USB
mirroring disrupted WDA's USB connection in earlier physical tests.

## Initial setup still required

- A signed, installed and trusted WebDriverAgent runner on the phone, with
  Developer Mode enabled and a valid signing profile.
- Apple's compatible Windows USB device stack, USB trust and developer pairing.
- The matching developer support image already mounted. The current manager
  does not download or mount it; phone reboot/update may require setup again.
- Private registration of the phone identifier and installed runner bundle ID.

For an already working setup, close iMirror and run the development-side command
with the actual identifiers from that setup:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/register-wda-runtime.ps1 -DeviceId "<device identifier>" -RunnerBundleId "<installed signed bundle identifier>" -PairingSource "<existing pairing directory>"
```

The placeholders are not real values. Registration stores `setup.json` and
pairing data under `%LOCALAPPDATA%\iMirror\wda`; these are private machine data,
never release assets or Git content. It does not read an Apple password, sign an
IPA, install an app, renew a profile or change Windows drivers. Without registered
setup, the manual advanced client at `http://127.0.0.1:8100` remains available.

## Implementation and build

`crates/input-wda/src/runtime.rs` owns a background supervisor. It starts the
staged go-ios tunnel and runner plus the small native forwarder. Windows Job
Objects terminate only its owned helpers on shutdown. Listener addresses are
loopback, with ports 28100 for tunnel information and 8100 for WDA. Occupied
ports produce an error; unrelated processes are never terminated.

The health check runs every two seconds with a 600 ms HTTP timeout. Helper exit
or repeated failed checks invalidates runtime readiness. Restarts use bounded
backoff; a changed runtime generation discards the old application client and
geometry before a new session can become ready. Input received while not ready
is rejected, and ambiguous failed taps are not replayed. Runtime readiness and
application-session readiness are separate diagnostics fields: an answering
server alone does not mean clicks are enabled.

Diagnostic helper messages are bounded and filtered. Normal diagnostics omit
the private setup fields. The forwarder binds `127.0.0.1`, caps concurrent TCP
clients at eight, resolves the current USBMux device ID on each new connection,
and refreshes idle deadlines while traffic is active.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-wda-runtime.ps1 -Destination "D:\path\to\staging\WDA"
```

The build script downloads hash-pinned Go 1.26.8 and the known-working official
go-ios Windows release asset, tests/builds the forwarder, and stages dependency
notices. End users run native EXEs and do not need Go installed. The release
stager includes this folder; the full installer has not been rebuilt or validated
for this candidate.

Forwarder source is GPL-3.0-only; go-ios is MIT. The official go-ios v1.3.2 asset's
embedded metadata reports a modified upstream revision, not an exact clean build
of the v1.3.2 source tag. `WDA/runtime-manifest.json` retains its actual build
metadata, hashes and dependency versions. Do not claim source reproducibility
for that upstream EXE; provenance and corresponding-source review remain public
release gates. No Apple binaries, signed phone runner or private pairing files
are included in the staged runtime.

## Exact candidate and evidence

- Candidate: `dist/wda-managed-20260913/iMirror.exe`, 3,325,952 bytes.
- SHA-256: `814fd334feca996971f710b6de90805bf6607611fe69c0a4d2da1437e48c60ee`.
- Phone: iPhone15,4, iOS 27.0; Windows build 26100.
- Software: fmt, strict all-target/all-feature Clippy, 67 Rust tests, release
  build and the Go forwarder test passed. PE analysis found only system imports
  for both WDA helper EXEs; this is not a clean-machine installation test.
  The runtime staging script also completed under Windows PowerShell 5.1 using
  cached dependencies; this was not a fresh offline/source-only build.
- Cold start: the candidate started its own tunnel/forwarder/runner and became
  ready without pre-existing manual helpers.
- Recovery: terminating only the candidate's forwarder produced Not Ready,
  followed by Ready with generation 1 → 3 in approximately 7.5 seconds. This
  test sent no phone input; evidence is in local `work/wda-setup/managed-recovery.json`.
- Close/reopen: the old main process exited and its children were gone. Its
  shutdown receipt recorded window hiding in 3.007 ms and loop exit in 193.162 ms
  with video inactive. The reopened app owned exactly its three WDA helpers.
- Physical acceptance: asked the user to close, wait about two seconds, reopen
  this exact EXE without pressing Connect WDA, connect Wireless and click
  Calculator. Their reply was “oke được nhé”. Record automatic reopen plus
  physical click as PASS on this setup. A subsequent check confirmed both
  application Ready and WDA `/status` ready, without an application error.

USB/video/BLE/Raw Input/decoder/renderer source was unchanged by this WDA work.
The saved Wireless Quality preference was retained. No new FPS or end-to-end
latency measurement was made; see the previous [Quality and latency evidence](WDA_PERFORMANCE_VALIDATION.md).

The reopened session's existing counters later showed nine tap requests with
zero failures, mean software dispatch 939.429 ms and median 851.103 ms. This
uncontrolled sample is not a comparable performance benchmark; do not attribute
an input-latency improvement to runtime management. It does not measure the
physical display response.

Still untested: phone/PC reboot, fresh driver or developer-image setup, signing
expiry/renewal, sustained cable-loss recovery, WDA drag/typing and a clean-Windows
installer. This is an unsigned engineering candidate, not a public release.
