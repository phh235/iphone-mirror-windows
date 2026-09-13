# WDA runner timing experiment — not built or installed

This is an **uncompiled measurement patch**, not a faster runner, a production
backend, or continuous touch. It is not part of the Windows build or installer.
No CI workflow is enabled and no remote job has run.

Base: Appium WebDriverAgent v16.12.8, commit
`3e8aa7de81f254dbb0876baa9e9173c16b55b3a0`. The three changed upstream files were
checked against their pinned Git blob IDs, and `git apply --check` passed.
New code uses the upstream-compatible BSD-3-Clause license. Original notices
remain; the build copies the pinned upstream LICENSE with the artifact.

## Why measure the runner

Current desktop observations put local request preparation/queue work in tens
of microseconds while tap HTTP completion is hundreds of milliseconds. A status
request is not the same server work as a tap, so subtracting its time cannot
give exact XCTest latency. The pinned `FBRunLoopSpinner` has a 100 ms completion
wait interval, but reducing it without server/visual measurements would be a
speculative optimization. It may improve only the response tail.

This patch adds `POST /session/:id/imirror/actions`, with the same W3C `actions`
body and the same active-application selection, validation, event synthesis and
cool-off policy as the ordinary route. Successful responses add
`value.imirror_timing_ms`, relative to a monotonic handler origin:

| Difference | Meaning |
|---|---|
| handler_start → active_application_ready | Current application lookup |
| synthesizer_begin → synthesizer_ready | Synthesizer initialization |
| synthesizer_ready → event_record_ready | Coordinate resolution / event-path construction |
| completion_wait_begin → synthesis_submit | Pre-submission work |
| synthesis_submit → synthesis_callback | XCTest/daemon completion as observed by the runner |
| synthesis_callback → completion_wait_return | Run-loop completion tail |
| cooloff_begin → cooloff_complete | Existing animation cool-off |
| handler_start → handler_complete | Measured handler scope |

It does **not** measure HTTP parsing before routing, response transfer, exact
network/tunnel time, phone photons, or Windows display time. The desktop ETW/ROI
measurement is still needed. Failed requests retain the normal WDA error response
and do not have a success timing object.

The ordinary `/actions` route is retained. The patch adds three lightweight
thread-context lookups there; marks are no-ops without a timing context. Compare
patched ordinary `/actions` against the unpatched runner to measure that
overhead before crediting any optimization. Use one in-flight gesture during
profiling. The callback captures the request's timing object explicitly, and a
lock protects the bounded set of timestamps across callback/request threads.
No coordinates, typed text, session identifiers or credentials enter the trace.

No change to the 100 ms spinner, gesture duration, transport, notification policy,
or touch state has been made. In particular, no mini-swipe or multi-request
persistent-touch approximation is introduced.

## Build / next validation boundary

On a Mac with a compatible Xcode selected:

```sh
bash experiments/wda-timing/build-macos.sh /tmp/imirror-wda-timing-run1
```

The output directory must be fresh; the script never deletes a prior build.
It pins the upstream revision, applies the patch, builds for a generic iOS
device with signing disabled, and packages an unsigned IPA. It must actually
compile before this experiment can be called implemented/testable. Windows
cannot execute this build. GitHub macOS CI is an alternative, but this private
repository can incur runner charges; no such job has been started.

The user must sign/install the resulting runner themselves using a legitimate
Apple identity/profile. Preserve the currently working signed runner and its
bundle/setup information before any replacement. The script does not read keys,
sign, install, alter Developer Mode, or modify Windows's registered WDA setup.

Once a signed experimental runner is available: verify `/status`, compare one
standard and timed Calculator tap, then run >=100 samples on each route with
real result checks and ETW ROI coverage. Only then consider a separate bounded
completion-wait or direct-coordinate experiment. Continuous-touch declarations
in XCTest headers still require separate continuity, cancellation, disconnect
and UP-delivery tests; this patch proves none of those.
