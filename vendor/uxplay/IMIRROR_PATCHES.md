# iMirror changes to pinned UxPlay

Upstream: https://github.com/FDH2/UxPlay

Pinned revision: `f2c4a66e704859e791e139db4f3fdba79cd838f9`.
The upstream GPL-3.0-or-later license and notices apply to this modified source.

Existing integration adds Windows named-event shutdown and notification of the
last AirPlay client disconnect. Those changes are unchanged by the following fix.

## RTP source timestamps (2026-09-12)

Affected file: `renderers/video_renderer.c`.

In the `-vrtp` path, sink synchronization is disabled. Previously both source
timestamp normalization and `GST_BUFFER_PTS` assignment depended on that flag.
Our real iPhone capture observed 496 different coded payload groups but one RTP
timestamp; iMirror's bounded timestamp deduplication consequently emitted only
the first picture.

Keep presentation synchronization unchanged and independently enable source PTS
for RTP forwarding. Reuse upstream's existing source-to-pipeline clock conversion.
Do not change non-RTP operation, the decoder, or the consumer's duplicate guard.

Rebuild only the helper from the repository root using PowerShell 7:

```powershell
pwsh -NoProfile -File scripts/build-airplay-helper.ps1
```

Output: `work/airplay-helper-build/build/uxplay.exe`. The script uses the existing
`docs/msys-build-lock.json`, verifies package SHA-256 values, extracts a local
UCRT SDK and records build inputs. Stage it with the matching private AirPlay
runtime; the EXE alone is insufficient. No package installation hooks run.

Native build and receiver startup/shutdown pass. The first patched phone capture
received 596 distinct timestamps and 596 coded payload groups in 15.001 seconds,
without sequence gaps. However, the user reported no picture in the application;
this is not successful live-video acceptance.

## Preserve the original source time on renderer retry (2026-09-12)

Affected file: `uxplay.cpp`, `video_process` callback.

The renderer can request an offset adjustment when the first source timestamp
precedes the pipeline base time. The existing callback modified the remote
timestamp in place, then added the entire clock offset again on retry. The
first captured RTP timestamp consequently differed sharply from following
timestamps. Save the original timestamp before the retry loop and calculate
each attempted presentation timestamp from that original value.

`scripts/test-airplay-clock.ps1` compiles the actual pinned callback with a
simulated renderer retry contract. Before the fix it delivered PTS 20 seconds,
then 16.64 milliseconds for consecutive frames; after the fix these are 0 and
16.64 milliseconds. It also checks the ordinary no-retry source delta. This is
a software clock regression test, not an iPhone integration test.

The second helper candidate builds and passes the clock regression test and
receiver startup/shutdown. Physical live-video validation remains pending. See
`docs/ENGINEERING_LOG.md` at the repository root for results. Existing upstream
`-march=native` must be evaluated separately before publishing a binary intended
for other CPUs.
