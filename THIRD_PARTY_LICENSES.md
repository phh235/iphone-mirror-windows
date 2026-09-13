# Third-party licenses

This is the **source dependency index**, generated from Cargo.lock and the pinned
components retained in this repository. It includes build/proc-macro dependencies;
it is not a claim that every listed crate is loaded at runtime. License expressions
are reproduced as declared by the package manifests.

iMirror is [GPL-3.0-only](LICENSE). Binary packages must also include the exact
runtime inventory, full notices and corresponding source required for their
contents. See [distribution requirements](docs/LICENSING.md). The release script
generates a separate package-specific inventory under `licenses/`.

## Retained native code, assets and build components

| Component | Revision/source | Terms and notice |
| --- | --- | --- |
| iPhoneMirror native core | `2635a0073c67539aec86487844fc41a5caddf9ba` | [GPL-3.0-only](vendor/iphone-mirror/LICENSE); [local patches](vendor/iphone-mirror/PATCHES.md) |
| UxPlay | `f2c4a66e704859e791e139db4f3fdba79cd838f9` | [GPL-3.0-or-later](vendor/uxplay/LICENSE); retained component notices also apply |
| UxPlay llhttp | Pinned UxPlay tree | [MIT](vendor/uxplay/lib/llhttp/LICENSE-MIT) |
| UxPlay PlayFair | Pinned UxPlay tree | [GPL notice](vendor/uxplay/lib/playfair/LICENSE.md) |
| UxPlay mdnsd | Pinned UxPlay tree | [LGPL-2.1-or-later header](vendor/uxplay/lib/mdnsd/mdnsd.c) |
| libusb | Pinned native-core third_party files | [LGPL-2.1-or-later](vendor/iphone-mirror/third_party/libusb/COPYING) |
| libusb-win32 | Pinned native-core third_party files | [LGPL-3.0](vendor/iphone-mirror/third_party/libusb-win32/COPYING_LGPL.txt) |
| QuickTime protocol fixtures | See fixture provenance | [MIT](vendor/iphone-mirror/src/Core/tests/fixtures/quicktime_video_hack/LICENSE); [provenance](vendor/iphone-mirror/src/Core/tests/fixtures/README.md) |
| windows-ble-hid report-design reference | `9a4f45129779ec3bef368ea4ffcdcde795a57b65` | [MIT](vendor/licenses/windows-ble-hid-MIT.txt) |
| Microsoft Fluent System Icons | `9cf8af0f95a555918a60b8147a2f33a6a1248442` | [MIT](assets/fluent/LICENSE), [NOTICE](assets/fluent/NOTICE), [asset hashes](assets/fluent/SHA256SUMS.txt) |
| WiX build/bootstrapper components | 3.14.1 | [Retained WiX license and exceptions](vendor/licenses/wix3/LICENSE.TXT) |
| Microsoft VC runtime | Official app-local deployment, selected at package build | [Redistribution documentation](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution); runtime DLLs are not checked into this source repository |

GStreamer, minimal FFmpeg audio libraries and other private wireless runtime
dependencies are rebuilt/staged by the packaging scripts. Audio playback in
iMirror is disabled; that does not make retained runtime license obligations
disappear. Exact native package archives are pinned in
[native-source-lock.json](docs/native-source-lock.json) and build packages in
[msys-build-lock.json](docs/msys-build-lock.json). Their staged-file hashes and
notices must be regenerated for an actual release, not inferred from this table.

The optional managed WDA runtime stages the MIT-licensed go-ios Windows CLI and
the GPL-3.0-only [iMirror forwarder](vendor/wda-forwarder/README.md). The forwarder
uses go-ios v1.3.2 through its locked Go module graph. Its staging script copies
the Go SDK and dependency licenses into `WDA/licenses` and records exact binary
hashes and embedded module versions in `WDA/runtime-manifest.json`. The official
CLI asset reports modified upstream source; it is hash-pinned, not claimed to
be rebuilt from the source tag. See [provenance and release limits](docs/WDA_MANAGED_RUNTIME.md).

The signed WebDriverAgent phone runner remains a separately installed prerequisite,
not a bundled app. Apple proprietary software, credentials, pairing records and
non-OSI iUsbBridge components are excluded.

## Rust packages from locked Windows x64 metadata

| Package | Version | Declared license | Upstream |
| --- | --- | --- | --- |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/atomic-waker) |
| base64 | 0.22.1 | MIT OR Apache-2.0 | [source](https://github.com/marshallpierce/rust-base64) |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | [source](https://github.com/bitflags/bitflags) |
| bitstream-io | 2.6.0 | MIT/Apache-2.0 | [source](https://github.com/tuffy/bitstream-io) |
| bytes | 1.12.1 | MIT | [source](https://github.com/tokio-rs/bytes) |
| cc | 1.2.56 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/cc-rs) |
| crossbeam-channel | 0.5.15 | MIT OR Apache-2.0 | [source](https://github.com/crossbeam-rs/crossbeam) |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | [source](https://github.com/crossbeam-rs/crossbeam) |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | [source](https://github.com/yaahc/displaydoc) |
| find-msvc-tools | 0.1.12 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/cc-rs) |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url) |
| four-cc | 0.4.0 | MIT/Apache-2.0 | [source](https://github.com/dholroyd/four-cc) |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| h264-reader | 0.8.0 | MIT/Apache-2.0 | [source](https://github.com/dholroyd/h264-reader) |
| hex-slice | 0.1.4 | MIT | [source](https://github.com/cstorey/hex-slice/) |
| http | 1.5.0 | MIT OR Apache-2.0 | [source](https://github.com/hyperium/http) |
| http-body | 1.1.0 | MIT | [source](https://github.com/hyperium/http-body) |
| http-body-util | 0.1.5 | MIT | [source](https://github.com/hyperium/http-body) |
| httparse | 1.10.1 | MIT OR Apache-2.0 | [source](https://github.com/seanmonstar/httparse) |
| hyper | 1.11.1 | MIT | [source](https://github.com/hyperium/hyper) |
| hyper-util | 0.1.20 | MIT | [source](https://github.com/hyperium/hyper-util) |
| icu_collections | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_locale_core | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_normalizer | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_properties | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_properties_data | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_provider | 2.3.1 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| idna | 1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url/) |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | [source](https://github.com/hsivonen/idna_adapter) |
| ipnet | 2.12.2 | MIT OR Apache-2.0 | [source](https://github.com/krisprice/ipnet) |
| itoa | 1.0.18 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/itoa) |
| libc | 0.2.189 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/libc) |
| litemap | 0.8.3 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| log | 0.4.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/log) |
| memchr | 2.8.3 | Unlicense OR MIT | [source](https://github.com/BurntSushi/memchr) |
| mio | 1.2.3 | MIT | [source](https://github.com/tokio-rs/mio) |
| mp4ra-rust | 0.3.0 | MIT/Apache-2.0 | [source](https://github.com/dholroyd/mp4ra-rust) |
| mpeg4-audio-const | 0.2.0 | MIT/Apache-2.0 | [source](https://github.com/dholroyd/mpeg4-audio-const) |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | [source](https://github.com/matklad/once_cell) |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url/) |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/pin-project-lite) |
| potential_utf | 0.1.6 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/proc-macro2) |
| quote | 1.0.47 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/quote) |
| reqwest | 0.12.28 | MIT OR Apache-2.0 | [source](https://github.com/seanmonstar/reqwest) |
| rfc6381-codec | 0.2.0 | MIT/Apache-2.0 | [source](https://github.com/dholroyd/rfc6381-codec) |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | [source](https://github.com/dtolnay/ryu) |
| serde | 1.0.228 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_core | 1.0.228 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_derive | 1.0.228 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/json) |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | [source](https://github.com/nox/serde_urlencoded) |
| shlex | 1.3.0 | MIT OR Apache-2.0 | [source](https://github.com/comex/rust-shlex) |
| slab | 0.4.12 | MIT | [source](https://github.com/tokio-rs/slab) |
| smallvec | 1.16.0 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-smallvec) |
| socket2 | 0.6.5 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/socket2) |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | [source](https://github.com/storyyeller/stable_deref_trait) |
| syn | 2.0.119 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/syn) |
| syn | 3.0.5 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/syn) |
| sync_wrapper | 1.0.2 | Apache-2.0 | [source](https://github.com/Actyx/sync_wrapper) |
| synstructure | 0.13.2 | MIT | [source](https://github.com/mystor/synstructure) |
| thiserror | 2.0.18 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| thiserror-impl | 2.0.18 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| tinystr | 0.8.4 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| tokio | 1.53.1 | MIT | [source](https://github.com/tokio-rs/tokio) |
| tower | 0.5.3 | MIT | [source](https://github.com/tower-rs/tower) |
| tower-http | 0.6.11 | MIT | [source](https://github.com/tower-rs/tower-http) |
| tower-layer | 0.3.3 | MIT | [source](https://github.com/tower-rs/tower) |
| tower-service | 0.3.3 | MIT | [source](https://github.com/tower-rs/tower) |
| tracing | 0.1.44 | MIT | [source](https://github.com/tokio-rs/tracing) |
| tracing-attributes | 0.1.31 | MIT | [source](https://github.com/tokio-rs/tracing) |
| tracing-core | 0.1.36 | MIT | [source](https://github.com/tokio-rs/tracing) |
| try-lock | 0.2.5 | MIT | [source](https://github.com/seanmonstar/try-lock) |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | [source](https://github.com/dtolnay/unicode-ident) |
| url | 2.5.8 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url) |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | [source](https://github.com/hsivonen/utf8_iter) |
| want | 0.3.1 | MIT | [source](https://github.com/seanmonstar/want) |
| windows | 0.61.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-collections | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-core | 0.61.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-future | 0.2.1 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-link | 0.1.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-numerics | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-result | 0.3.4 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-strings | 0.4.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-threading | 0.1.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| writeable | 0.6.4 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| yoke | 0.8.3 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| yoke-derive | 0.8.2 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerofrom | 0.1.8 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerofrom-derive | 0.1.7 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerotrie | 0.2.5 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerovec | 0.11.8 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerovec-derive | 0.11.6 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zmij | 1.0.23 | MIT | [source](https://github.com/dtolnay/zmij) |

Regenerate with `pwsh -File scripts/update-license-index.ps1`; verify with `-Check`.
Full third-party notices for binary distributions are collected by `scripts/runtime-inventory.py`.
Preserve the special upstream notice handling recorded under `vendor/licenses/`.
