# Contributing to iMirror

Start with the [user guide](docs/USER_GUIDE.md), [build instructions](docs/BUILDING.md)
and [architecture](docs/ARCHITECTURE.md). The project is an engineering preview;
help improve reproducibility and test evidence without overstating compatibility.

## Development workflow

1. Create a focused branch from `main`.
2. Keep changes limited to the issue being addressed. Preserve the working USB,
   decode/render and Bluetooth/Raw Input paths unless the change requires them.
3. Run formatting, strict Clippy, tests and a release build. Add tests for
   meaningful behavior changes; do not substitute mocks for real-device evidence.
4. Describe the problem, the resulting behavior, validation and known limitations
   in the pull request. Separate software checks from physical iPhone tests.

Rust 1.96.0 MSVC is the currently tested toolchain. The manifest's minimum Rust
version is not a claim that every supported compiler/Windows/iOS combination has
been validated. Do not add UI frameworks or runtime dependencies without a
measured benefit and a licensing/packaging plan.

## Coding and native boundaries

- Use typed errors in libraries and handle failures at app boundaries. Avoid
  `unwrap`, `expect`, silent error suppression and unbounded queues in runtime code.
- Every unsafe block must explain its invariants with a `SAFETY` comment.
- Keep input, device/network work and video processing off the UI paint path.
- Keep ownership explicit. Cache resources where useful, but do not add caching
  or low-level complexity without evidence.
- Preserve upstream code and licenses in `vendor/`. Record necessary upstream
  patches, pins and reasons instead of reformatting whole vendor trees.
- Leave superseded experiments in Git history; do not keep a second inactive
  implementation in the production source tree.

## Test reports and privacy

For hardware tests, record build revision/hash, Windows build, phone model/iOS,
transport, source resolution/FPS and the actual operation tested. Report failures,
disconnects and cancelled runs. A successful GATT notification is not proof that
the physical phone acted, and render submissions are not source FPS.

Keep raw local evidence under ignored `work/` or `benchmarks/`. Review diagnostics
before sharing. Never commit credentials, signing certificates, provisioning or
pairing material, private device identifiers, developer-specific paths or user
screen recordings. Use [SECURITY.md](SECURITY.md) for vulnerability reports.

## Commits and distribution

Do not commit build caches, packaged binaries, downloaded toolchains or local
configuration. The checked-in app ICO and pinned native import/runtime files are
intentional source assets, not a reason to add arbitrary binaries.

Contributions to iMirror are provided under [GPL-3.0-only](LICENSE), while third-party
material retains its applicable license. Preserve attribution and identify any
new dependency. Public binary release requires the exact runtime notices,
corresponding source, hardware acceptance and clean-Windows validation described
in [LICENSING.md](docs/LICENSING.md) and [VALIDATION.md](docs/VALIDATION.md).
