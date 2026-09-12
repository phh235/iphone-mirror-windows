# Microsoft Fluent System Icons (Regular, 20)

Official upstream: https://github.com/microsoft/fluentui-system-icons

Pinned commit: `9cf8af0f95a555918a60b8147a2f33a6a1248442`.
License: MIT, copyright Microsoft Corporation. `LICENSE` and upstream `NOTICE`
are included unchanged. `SHA256SUMS.txt` records the exact downloaded files.

Only eight SVGs are vendored. Files are unchanged from upstream
`assets/<Title Case Name>/SVG/ic_fluent_<name>_20_regular.svg`:

| Action | Upstream folder / SVG name |
| --- | --- |
| Connect | Plug Connected / plug_connected |
| Disconnect | Plug Disconnected / plug_disconnected |
| Rotate | Arrow Rotate Clockwise / arrow_rotate_clockwise |
| Control (all states) | Cursor Click / cursor_click |
| Fullscreen | Full Screen Maximize / full_screen_maximize |
| Exit fullscreen | Full Screen Minimize / full_screen_minimize |
| Settings | Settings / settings |
| Existing narrow-window overflow | More Horizontal / more_horizontal |

Rust embeds these SVG bytes. Windows Direct2D parses them once and extracts the
single, untransformed path into retained vector geometry. All use the default
nonzero fill rule. The original `#212121` paint is replaced at draw time with the
current UI foreground brush; path coordinates and optical weight are unchanged.
The app does not load the files from disk, use an icon font, or ship raster icons.

The DC render target, geometry and brush are cached. A temporary WARP device is
used only to access Windows' native SVG parser and is released after extraction;
it never shares a device, context, buffer or surface with iPhone video rendering.
