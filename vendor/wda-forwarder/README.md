# iMirror WDA forwarder

Native Go helper used only by the optional managed WDA backend. The main app
remains Rust/Win32. Protocol forwarding reuses go-ios v1.3.2; `go.mod` and
`go.sum` pin the dependency graph.

This helper binds only 127.0.0.1:8100, caps simultaneous connections at eight,
resolves the registered phone's current USBMux numeric ID for each new TCP
connection, and refreshes idle deadlines on activity. It does not sign/install
apps, capture video or expose a remote control listener. Upstream device logs
are suppressed so pairing/device identifiers do not enter normal diagnostics.

Build and test with `scripts/prepare-wda-runtime.ps1 -Destination <absolute path>`
using Windows PowerShell 5.1 or newer on the development machine. The script bootstraps a
checksum-pinned Go SDK, builds this helper with no C runtime dependency, stages
the checksum-pinned go-ios Windows CLI, and copies runtime dependency licenses.
End users do not need Go or PowerShell 7 to run the resulting native binaries.

The helper's own source is GPL-3.0-only under the repository LICENSE. go-ios is
MIT; dependency notices are generated in the runtime's `licenses` directory.
No Apple binaries, signed WDA IPA, credentials or pairing files are included.
