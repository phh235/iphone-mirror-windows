# iMirror changes to the pinned native core

Upstream: https://github.com/RayrenSX/iPhoneMirror
SHA: 2635a0073c67539aec86487844fc41a5caddf9ba
License: GPL-3.0-only, retained in LICENSE.

Only Core sources, wireless IPC header dependencies, and native USB dependencies
were imported. The WPF frontend, non-OSI iUsbBridge and proprietary Apple
binaries are excluded.

- Rust statically links Core through cc; add oleaut32 explicitly.
- Root CMake builds the native test subset.
- Expose all four existing renderer rotations through im_session_set_window_rotation
  without renegotiating USB. Upstream had suppressed 90/270 degree manual rotation.
- SourceFrameCounter counts timestamped source samples, de-duplicates a bounded
  256-sample recent window, accounts for multi-sample packets and epoch changes,
  and marks FPS unavailable when source timing is missing. This does not hash
  pixels: unchanged screen content at a new source timestamp remains a new frame.
- CaptureStatus.fps is -1 when source timing is incomplete; Rust maps it to None.
  Its latency_ms field remains local decode time, never end-to-end latency.

No source-frame or physical-device performance claim has been made from the
synthetic regression tests.

- Video-only audio guard: when play_audio is false, skip PCM output allocation
  and WASAPI initialization/enqueue. QuickTime audio packet parsing and liveness
  timestamps remain intact; video receive/decode/render/watchdog logic is unchanged.
