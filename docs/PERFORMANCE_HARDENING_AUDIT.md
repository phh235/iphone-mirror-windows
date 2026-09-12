# Performance and UI routing audit — before implementation

Baseline source: `52f8f1d`, tested-folder EXE version 0.1.0 (r2).
No implementation optimization has been applied when this report is created.
Raw evidence is under `work/performance-hardening/` (ignored by Git).

## Baseline boundary

Process 4428 was already running. A process-counter collector records working
set, private bytes, paged memory, handles, threads, CPU time and the application's
actual input statistics. The first 300 seconds are retained in
`baseline-before.json`. Native logs show discovery-only operation and the worker
enumerates devices only without a media session. BLE is ready with both report
subscriptions; capture was off. This is disconnected video with BLE enabled,
not an all-backends-off idle benchmark. Startup, GPU utilization and first-frame
timings are not inferred. Separate active USB, captured BLE and wireless runs
still need confirmed hardware state. Earlier measurements are historical only.

## First-click investigation

The preview posts WM_KILLFOCUS to the parent. The parent later calls
ReleaseCapture unconditionally. A normal BUTTON can acquire capture on its
mouse-down before that posted message runs; the delayed preview cleanup can
then release the BUTTON's capture before mouse-up. This is a concrete routing
defect identified in source, pending message-trace and physical-click confirmation.
The fix must release capture only if the preview still owns it, and ignore a
stale focus-loss message if focus has already returned to the preview.

Other risks: production WM_COMMAND ignores the notification high word; reentrant
notifications can be lost through try_borrow_mut; dialog-key routing must preserve
Space/Enter once-only activation. No custom hit-test or eat-activation policy is
present. Video host layout excludes the toolbar. BLE Raw Input is confined to
the viewport, and its dedicated release mechanism must remain unchanged.

DrawFocusRect is called whenever ODS_FOCUS is set. This ignores input modality
and explains the dotted focus rectangle after mouse clicks. Replace only its
visual with a stable rounded keyboard focus outline; native BUTTON owns clicks.

## Code and cost inventory

| Area | Current cost/issue | Proposed disposition |
|---|---|---|
| Inactive app/src/ui.rs | Not compiled; obsolete API calls and UI | Remove after preserving Git history; maintenance benefit only |
| Theme active state | Unconditional invalidation every UI update | Measure paint calls before considering state-change-only invalidation |
| Labels | Allocate UTF-16 Vec before comparing existing text | Measure allocations; no speculative change |
| Theme refresh | Constructs fonts/brush before detecting no change | Low-rate path; defer unless measured benefit |
| Settings updates | Repeated native state/text queries and messages at 5 Hz | Instrument; retain correctness before reducing work |
| UI/device snapshots | Device polling and cloned snapshots every 100 ms | Frozen behavior for the first-click pass; quantify before changing |
| Device discovery | Roughly 2-second scans when disconnected; native calls can take hundreds of ms | Real baseline already includes this; no change without discovery-latency comparison |
| Input accumulator | One motion slot; fixed-size reports, bounded 128 transitions | Preserve |
| BLE notifications | Small WinRT IBuffer/callback allocation per async notification | Required ownership; preserve until native profiling proves a safe benefit |
| BLE lookup | Cached hot path; cold subscription validation every 500 ms | Preserve correctness and cache invalidation |
| Input metrics | Fixed atomic rings; snapshot allocates/sorts periodically | No per-mouse-event file logging; measure collector cost |
| Diagnostics | Serializes compact and pretty JSON every second, even closed | Candidate for lazy/opt-in persistence, after comparable baseline |
| Host diagnostics | Eager startup probe on its own thread | Candidate for lazy collection, not a UI-thread move |
| WDA | Native HTTP client initialized only on explicit selection | Keep Advanced-only and lazy |
| Wireless | UxPlay starts on explicit selection | Preserve; no plugin deletion without hardware/clean-host validation |
| Config migration | Preserves existing settings and user sensitivity | Keep compatibility; old format is not automatically dead code |
| Direct Touch | Preserved in Git; no production transport | Do not resurrect; generic transform tests are not a live backend |

## Threads and synchronization

- UI Win32 message loop: 200 ms snapshot timer; no video decoding.
- Device worker: bounded command/snapshot channels, 100 ms command wait,
  disconnected discovery/backoff; owns media sessions.
- Raw Input thread: message-only HWND, global emergency shortcut, 50 ms
  foreground/capture guard; no GATT calls.
- BLE/control worker: signaled waitable timer, short accumulator mutex, cached
  notification completion wait; cold diagnostics share this thread.
- Diagnostic writer: bounded one-snapshot channel, file rotation and serialization.
- Native USB: receive thread, video decode worker, D3D presentation worker and
  logging flush worker; additional driver/COM/system threads are dynamic.
- Wireless when enabled: helper process plus stdout/stderr and RTP receive
  threads, native encoded decode/render workers.
- WDA's blocking HTTP client has its own internal runtime only when enabled.

The movement mutex never encloses GATT. UI RefCell is thread-local but Win32
reentrancy is relevant to lost commands. Native video locks protect shared
frames/queues and are high-risk; do not replace them with atomics by inspection.

## Video memory/copies — preserve pending profiling

USB encoded queue: 12 samples / 64 MiB cap, keyframe recovery on overflow.
Wireless: one RTP access unit capped at 8 MiB; three native queued units.
Decoded rendering uses a latest-frame mailbox and shared D3D11 textures.
CPU materialization/software fallback exists; allocation/copy counts are not
established merely by reading those fallback functions. No queue shrinking or
GPU path rewrite is justified by the present evidence. Media timer resolution
is acquired only while a media Session exists and balanced at destruction.

## Dependencies and release

Cargo metadata and feature trees are saved before edits. reqwest already disables
default features and enables blocking/json for optional loopback WDA. It brings
HTTP/URL/Unicode dependencies; replacing it solely for binary size is not justified.
windows/windows-future are required native/WinRT bindings; cc is build-only.
serde/serde_json/thiserror/crossbeam-channel/h264-reader/socket2/tracing have real
call sites. Proc-macro dependencies contribute build/source footprint rather than
installed runtime DLLs. No dependency has yet been removed.

The runtime still needs USB DLLs/helper, Microsoft app-local CRT and the private
wireless stack. Current installer recipes hardcode 0.1.0 and lack a complete
one-Setup release workflow/version resource/CI. Those are release correctness
work, not performance optimizations. The requested primary download will be
`iMirror-vX.Y.Z-windows-x64-setup.exe`; raw EXE remains a local engineering artifact.
Do not publish until exact-binary physical and clean-machine gates are met.

## Decisions before measurement

No runtime optimization is accepted yet. First-click/focus fixes are correctness
repairs. Candidate performance changes require before/after data and a rollback
if benefits are negligible or behavior regresses. 30-minute stability, GPU usage,
full startup breakdown, allocation profiling and active-mode comparisons remain
unmeasured for this pass. No success is inferred from compilation or appearance.
