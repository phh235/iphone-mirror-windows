# Licensing and distribution

The application uses [GPL-3.0-only](../LICENSE). The original license text and
upstream notices are retained. [THIRD_PARTY_LICENSES.md](../THIRD_PARTY_LICENSES.md)
indexes the current source dependencies; it does not certify a packaged release.

## Material in the repository

- The pinned iPhoneMirror native core is GPL-3.0-only. Its retained files and
  patches are described in [PATCHES.md](../vendor/iphone-mirror/PATCHES.md).
- UxPlay carries GPL-3.0-or-later terms, with additional component notices under
  its `lib` directories. Reusing it through a helper process does not by itself
  remove applicable GPL obligations.
- libusb and libusb-win32 have LGPL terms in their retained COPYING files. The
  checked-in DLL/import libraries are intentional pinned dependencies.
- The Bluetooth report-design reference and Microsoft Fluent icons have MIT
  notices. The app logo is the user-supplied repository artwork.
- Rust dependencies carry the license expressions recorded in their manifests;
  the source index is generated from locked Windows-target Cargo metadata.
- The optional WDA forwarder is GPL-3.0-only and uses a locked Go module graph.
  Its staging script includes MIT go-ios and dependency notices under `WDA`.
  The official CLI binary's modified-source metadata and outstanding provenance
  review are documented in [WDA_MANAGED_RUNTIME.md](WDA_MANAGED_RUNTIME.md).
- WiX and Microsoft runtime files retain separate licensing terms. Microsoft
  redistributables are staged only from the official Visual Studio redistribution
  directory; the source repository does not contain those runtime DLLs.

## Before distributing binaries

1. Build and identify the exact application, helpers and private runtime. Keep
   hashes, source revisions, native patches, Cargo.lock and build recipes together.
2. Generate the staged runtime inventory and copy all applicable copyright,
   license and attribution notices. Resolve missing notices before redistribution.
3. Supply the corresponding source required by the applicable GPL/LGPL licenses,
   including modifications and the scripts needed to build the covered software.
   `scripts/package-source.py` assembles pinned native archives and Cargo sources;
   inspect and test the resulting archive instead of assuming generation proves
   completeness. Do not substitute an unrelated upstream source URL for the
   source matching modified distributed binaries.
   The new forwarder's Go dependency sources also need corresponding-source
   coverage; the existing Cargo/native archive script alone does not include
   that Go dependency graph.
4. Preserve the ability to replace applicable LGPL shared libraries. Keep official
   Microsoft redistribution notices and comply with their redistribution terms.
5. Do not include Apple proprietary drivers/binaries, non-OSI iUsbBridge files,
   signing identities, provisioning profiles, pairing records or user credentials.

The repository's source inventory and a successful compile do not complete these
steps. Clean-machine packaging, physical acceptance, final notice review and a
matching source archive remain release gates in [VALIDATION.md](VALIDATION.md).

Primary texts are the [GPL license](../LICENSE), the individual upstream license
files linked in the inventory, and Microsoft's
[application-local deployment documentation](https://learn.microsoft.com/en-us/cpp/windows/walkthrough-deploying-a-visual-cpp-application-to-an-application-local-folder).
Do not change the project's license or remove upstream attribution as a cleanup.

## Names and affiliation

iMirror is an independent project. It is not affiliated with, endorsed by or
sponsored by Apple Inc. or Microsoft Corporation. Product names and trademarks
belong to their respective owners; referring to a protocol or platform does not
grant permission to redistribute its proprietary software.
