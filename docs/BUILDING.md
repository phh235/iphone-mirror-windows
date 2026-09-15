# Building and checking iMirror

These are development instructions. Users of a complete app package do not need
the build tools listed here. Start with the [user guide](USER_GUIDE.md) for using
the app, and [validation status](VALIDATION.md) before describing a build as a release.

## Native app prerequisites

- Windows x64; Windows 11 is the primary development platform.
- Rust with the MSVC toolchain, Cargo, rustfmt and Clippy. Rust 1.96.0 is the
  currently tested version.
- Visual Studio 2022 C++ Build Tools with the desktop C++ workload and Windows
  SDK, including the resource compiler. Do not use the GNU Rust target for this build.
- Git to clone the repository; pinned native source and USB libraries are included.

```powershell
git clone https://github.com/phh235/iphone-mirror-windows.git
cd iphone-mirror-windows
rustup component add rustfmt clippy
cargo build
cargo build --release
```

Cargo compiles the Rust workspace, pinned C++ media core, USB configuration
helper and app icon resources. `target/release` contains `iMirror.exe`,
`iPhoneMirror.UsbConfigurationSwitch.exe`, `libusb-1.0.dll` and `libusb0.dll`.
This directory alone is not a validated distributable: USB DLL runtime requirements
and the optional private wireless runtime must be staged correctly for end users.

Optional managed WDA also needs the `WDA` directory beside the EXE. Build it with
`scripts/prepare-wda-runtime.ps1 -Destination <staging-directory>/WDA`; this
bootstraps a pinned Go SDK on the build host, not on the end user's machine.
See [initial phone setup and runtime provenance](WDA_MANAGED_RUNTIME.md).

The app logo comes from the checked-in ICO, so ordinary builds do not run an
image converter. See [app icon provenance](../assets/APP_ICON.md) and
[Fluent asset provenance](../assets/fluent/README.md) when changing artwork.

## Required checks

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
pwsh -File scripts/check-docs.ps1
```

GitHub Actions runs the source checks and Windows build. CI has no physical
iPhone and cannot certify USB, Bluetooth, wireless or clean-machine usability.
No workflow automatically publishes a release from these checks.

## Package an already validated portable build

The portable packager preserves the exact EXE and runtime bytes from a named
hardware-tested staging folder. It checks the expected EXE hash, recorded Rust
source hashes, native imports and ZIP CRCs. Use Python 3.12+ and MSVC dumpbin on
the packaging host only. Output must be a new directory under dist; old stages
and checkpoints are never overwritten.

```powershell
python scripts/package-portable.py --validated-stage dist/wda-recovery-20260915 --expected-exe-sha256 4aaffa975e13580c584fa9d1974952367e7a91713ab151fa56cee1b1cd2fc6c3 --output-dir dist/release-0.1.0-preview.1 --dumpbin "C:\path\to\MSVC\bin\Hostx64\x64\dumpbin.exe"
python scripts/package-source.py --runtime dist/release-0.1.0-preview.1/iMirror --go "C:\path\to\pinned-Go\bin\go.exe" --output dist/release-0.1.0-preview.1/iMirror-v0.1.0-corresponding-source.zip
```

The Go compiler version is pinned by prepare-wda-runtime.ps1. Package source
after committing intended source changes: its inventory is git ls-files, excluding
work, dist, pairing and credentials. Untracked source is deliberately not guessed.
Native source archives and module sources are downloaded and hash/lock checked.
The separate official go-ios MIT EXE still has its disclosed dirty-revision limit.

Extract the final ZIP, verify files.sha256.json and run scripts/verify-package.ps1
with PackageDirectory pointing to the extracted iMirror folder. Use a temporary
LOCALAPPDATA/TEMP and a Windows-only PATH for isolation. A test on a development
host is not a clean-machine test. Keep the GitHub Release as a draft until its
clean-Windows gates have been checked; do not silently label a preview stable.

For the C++ protocol/native tests, install CMake and run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/native-tests.ps1
```

The C++ tests supplement hardware testing. Any test requiring an actual phone
must explicitly record its setup, revision and result; never replace it with a
mock and call hardware validation complete.

## Engineering diagnostics

Most users should use **Settings → Advanced → Copy Diagnostics**. Developers can
also run the following against a complete app directory:

```powershell
iMirror.exe --version
iMirror.exe --list-devices
iMirror.exe --diagnostics --output host-diagnostics.json
iMirror.exe --ui-icon-benchmark
```

An app-owned UI smoke check is `iMirror.exe --ui-smoke-test`. It does not establish
physical click reliability. Detailed button tracing is opt-in through the
`ui-input-trace` Cargo feature and `--ui-button-trace <path>`; production builds
do not enable that feature by default. Keep private diagnostics out of Git.

Real-device measurement, with the phone unlocked and kept awake:

```powershell
iMirror.exe --benchmark usb --render --seconds 30 --output usb-test.json
```

The benchmark writes JSON and adjacent progress/sample files. Closing its window
cancels the run. A received-frame count, decode duration, GATT completion or
render submission is not an end-to-end physical latency result.

## Full runtime and installer build

Additional development prerequisites: CMake and Python 3.11+. Internet access is
needed the first time the hash-pinned MSYS/native build tools, source archives
and WiX toolset are fetched. They live under ignored `work/`, not in the installed
app. Their versions/hashes are recorded in the lock JSON files under `docs/`.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1
```

The script runs checks, builds the private wireless runtime, stages official
Microsoft application-local VC runtime files, collects notices and creates MSI,
Setup EXE, portable ZIP and corresponding-source artifacts under `dist/`.
Those artifact names and staging details are build-script outputs, not evidence
that a public release has been approved. The intended normal download is the
Setup EXE; installer unification/signing/clean-machine gates remain tracked in
[VALIDATION.md](VALIDATION.md).

`-SkipAirPlayBuild` requires an already built compatible private runtime.
`-SkipChecks` is only for local package iteration and does not satisfy release
validation. The current retained runtime may predate later source work; do not
copy an old preview into a new release and call it freshly validated.

Use [LICENSING.md](LICENSING.md) before binary redistribution. The generated
runtime inventory is specific to the staged files; the repository-level
[license index](../THIRD_PARTY_LICENSES.md) is not its replacement.

## Cleanup

Build metadata and native development tools can occupy several GiB. With a
complete staged app under `dist/`, use PowerShell 7:

```powershell
pwsh -File scripts/clean.ps1 -Preview
pwsh -File scripts/clean.ps1
```

The script selects the newest staged app with a matching build manifest, or
accepts `-KeepRelease <directory-under-dist>`. It validates targets, refuses
tracked/in-use/reparse paths, preserves source/vendor/research/Git history and
archives bulky local test evidence with per-file SHA-256 verification before
removal. Small diagnostic JSON/log/script records remain accessible. The next
build regenerates its outputs and may download native build packages again.

Do not delete the native files under `vendor/` or the DLLs beside an app you intend
to run. A clean checkout is source, not a portable application folder.
