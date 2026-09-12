# Security reporting

iMirror is an engineering preview. A security update policy for supported public
releases will be documented when those releases exist.

If GitHub private vulnerability reporting is available in this repository's
Security tab, use it. Otherwise open a minimal issue asking the maintainer for a
private reporting channel, without publishing exploit details, credentials or
pairing material. No private-reporting feature or dedicated email address is
assumed to be configured.

Include the affected revision, Windows/iOS versions, prerequisites, reproduction
steps and impact in the private report. Use synthetic data where possible. Never
include Apple credentials, private keys, provisioning profiles, lockdown pairing
records, private screen content or personally identifying device records.

Local control endpoints should remain loopback-only. Optional wireless discovery
and mirroring use the selected local network. Native packet parsers, HID report
handling, helper-process boundaries and installer/runtime provenance are useful
areas for review. A code or build check is not a claim that the app has completed
an independent security audit.
