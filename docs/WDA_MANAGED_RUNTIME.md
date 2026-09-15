# Managed WDA runtime

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
- A compatible Apple developer support image already mounted, or its local
  `Restore` directory registered for automatic preparation. The manager checks
  image state on each startup/reconnect attempt and mounts the registered cache
  when missing. It does not download images or guess a replacement after an iOS update.
- Private registration of the phone identifier and installed runner bundle ID.

For an already working setup, close iMirror and run the development-side command
with the actual identifiers from that setup:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/register-wda-runtime.ps1 -DeviceId "<device identifier>" -RunnerBundleId "<installed signed bundle identifier>" -PairingSource "<existing pairing directory>" -DeveloperImagePath "C:\path\to\Apple-image\Restore"
```

The placeholders are not real values. Registration stores `setup.json` and
pairing data under `%LOCALAPPDATA%\iMirror\wda`; these are private machine data,
never release assets or Git content. It does not read an Apple password, sign an
IPA, install an app, renew a profile or change Windows drivers. Without registered
setup, the manual advanced client at `http://127.0.0.1:8100` remains available.

`-DeveloperImagePath` registers the existing local Apple image, not a bundled
asset. It must be a directory containing `BuildManifest.plist`; filesystem links
and network paths are rejected. Omitting the parameter while updating the same
phone's setup preserves an existing registration. The image files stay private
and are not copied into the installer. Older registrations still work if the
image is already mounted; otherwise an explicit missing-image message is shown.

## Recovery after restart

When WDA is enabled, the background supervisor checks Developer Mode and the
mounted-image signature list before starting the tunnel/runner. If the image is
missing, it uses the registered Restore directory, then checks the mounted state
again. Only after WDA's server and application session are established may Control
become Ready. A registered path by itself is not readiness.

Image preparation uses a bounded, cancellable native helper process. Queries
have a 10-second deadline; mounting has a 120-second deadline and may contact
Apple's personalization service. Closing the app cancels preparation through
its owned process Job. The UI, video and Raw Input threads do not perform the work.

Diagnostics expose image state and mount counters without image paths,
signatures or private pairing data. Known helper failures are classified into
unlock, Developer Mode, pairing, runner signing/installation, or image-setup
issues instead of treating all failures as a generic runner disconnect.

This cannot bypass phone passcodes, Trust prompts, Developer Mode, expired or
revoked signing profiles, or an incompatible image after an iOS update. Those
conditions need the corresponding user/setup action. Reboot recovery must be
validated on the exact build and phone; automatic preparation code alone is not
a hardware PASS.

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

## Developer-image recovery candidate (2026-09-15)

- Candidate: `dist/wda-recovery-20260915/iMirror.exe`, 3,490,304 bytes.
- SHA-256: `4aaffa975e13580c584fa9d1974952367e7a91713ab151fa56cee1b1cd2fc6c3`.
- fmt, strict all-target/all-feature Clippy, 82 default Rust tests, 26 WDA
  all-feature tests and release build passed. The 26 overlap the default suite;
  they are not 26 additional unique tests. Documentation checks also passed.
- With the image already mounted, this EXE became Ready with zero mount attempts.
- After the user reported restarting the iPhone and reopened the candidate,
  the new process reported one mount attempt, one successful mount, image
  `mounted`, runtime/application Ready and no startup issue. The user then
  confirmed a real Calculator click over Wireless: "Có, iPhone nhận click".
  Record automatic preparation plus physical click as PASS for this exact setup.
- The old process exited before the reboot observation. This tests a new app
  process after phone restart, not recovery while keeping the same app process
  open. Windows reboot, signing expiry, incompatible images, sustained recovery
  and clean-machine installation remain untested.
- Runtime binaries match the previous stage, except the new application EXE.
  Mirroring, BLE, Raw Input and rendering source are unchanged. Stable checkpoint
  and stable EXE are preserved. This is an unsigned test candidate.

## Earlier managed-runtime candidate and evidence (2026-09-13)

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

Untested on that earlier candidate: phone/PC reboot, fresh driver or developer-image setup, signing
expiry/renewal, sustained cable-loss recovery, WDA drag/typing and a clean-Windows
installer. This is an unsigned engineering candidate, not a public release.

### Subsequent failure under investigation

After the successful reopen test, the user reported another non-working click
sequence. The bounded history contains a completed tap followed by a WDA request
failure; new sessions then reset their counters to zero. The earlier PASS remains
evidence for that particular test, not proof of reliable operation. A separate
`dist/wda-click-trace-20260913` candidate retains click-path and failed-session
evidence. The user subsequently confirmed a working physical Calculator click
on that candidate, while reporting substantial delay. Sustained reliability is
still unvalidated; see the engineering log.
