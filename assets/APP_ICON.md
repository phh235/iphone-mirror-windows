# iMirror application logo

`logo.png` is the image supplied by the user. Its artwork, background and
placement are preserved. `imirror.ico` contains downsampled PNG icon images at
16, 20, 24, 28, 32, 40, 48, 56, 64, 96, 128 and 256 pixels.

To regenerate after changing the original image:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-app-icon.ps1
```

The conversion uses Windows System.Drawing on the development machine. Normal
Cargo builds use the checked-in ICO and the Windows SDK resource compiler,
located through the existing find-msvc-tools package. The installed app does
not require image conversion, PowerShell, .NET, or a loose PNG/ICO file.

Resource 101 is embedded in iMirror.exe. Native window icons are loaded at the
system small/large sizes for the window DPI and cached until application exit.
Windows Explorer uses the embedded resource. WiX shortcut/ARP and bootstrapper
icon configuration reference the same ICO. Toolbar actions retain Fluent SVGs.
