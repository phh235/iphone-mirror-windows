//! Pinned Fluent SVGs parsed by Windows once into device-independent vector geometry.
//! Only icon rendering uses this factory/DC target; the video D3D device is never accessed.
use crate::toolbar::Icon;
use std::{cell::RefCell, time::Instant};
use windows::{
    Win32::{
        Foundation::{COLORREF, E_FAIL, HMODULE, RECT},
        Graphics::{
            Direct2D::{Common::*, *},
            Direct3D::D3D_DRIVER_TYPE_WARP,
            Direct3D11::*,
            Dxgi::{Common::DXGI_FORMAT_B8G8R8A8_UNORM, IDXGIDevice},
            Gdi::*,
        },
        System::{ProcessStatus::*, Threading::GetCurrentProcess},
        UI::Shell::SHCreateMemStream,
    },
    core::{Error, Interface, Result, w},
};

const SOURCES: [&[u8]; 8] = [
    include_bytes!("../../../assets/fluent/ic_fluent_plug_connected_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_plug_disconnected_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_arrow_rotate_clockwise_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_cursor_click_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_full_screen_maximize_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_full_screen_minimize_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_settings_20_regular.svg"),
    include_bytes!("../../../assets/fluent/ic_fluent_more_horizontal_20_regular.svg"),
];
thread_local! {
    static CACHE:RefCell<Option<Result<Cache>>>=const { RefCell::new(None) };
}
struct Target {
    dc: ID2D1DCRenderTarget,
    brush: ID2D1SolidColorBrush,
}
struct Cache {
    factory: ID2D1Factory1,
    paths: Vec<ID2D1PathGeometry1>,
    target: Option<Target>,
    load_us: f64,
    segments: u32,
}
fn invalid(message: &'static str) -> Error {
    Error::new(E_FAIL, message)
}
impl Cache {
    fn new() -> Result<Self> {
        let started = Instant::now();
        // SAFETY: All COM resources stay on this UI thread. The temporary WARP
        // device only enables the OS SVG parser and is dropped after geometry extraction.
        unsafe {
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let mut device = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )?;
            let device = device.ok_or_else(|| invalid("No SVG parsing device"))?;
            let dxgi: IDXGIDevice = device.cast()?;
            let d2d = factory.CreateDevice(&dxgi)?;
            let context: ID2D1DeviceContext5 = d2d
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?
                .cast()?;
            let mut paths = Vec::with_capacity(SOURCES.len());
            let mut segments = 0;
            for source in SOURCES {
                // Only these trusted, embedded, single-path 20x20 assets are accepted.
                // No external SVGs, CSS, images, transforms or network resources are loaded.
                validate_source(source)?;
                let stream = SHCreateMemStream(Some(source))
                    .ok_or_else(|| invalid("SVG memory stream failed"))?;
                let svg = context.CreateSvgDocument(
                    &stream,
                    D2D_SIZE_F {
                        width: 20.0,
                        height: 20.0,
                    },
                )?;
                let root = svg.GetRoot()?;
                let path = root.GetFirstChild()?;
                let data: ID2D1SvgPathData = path.GetAttributeValue(w!("d"))?;
                // All pinned files use the SVG default nonzero fill rule. The
                // upstream #212121 paint is deliberately replaced by the UI brush.
                let geometry = data.CreatePathGeometry(D2D1_FILL_MODE_WINDING)?;
                segments += geometry.GetSegmentCount()?;
                paths.push(geometry);
            }
            let target = Some(Target::new(&factory)?);
            Ok(Self {
                factory,
                paths,
                target,
                load_us: started.elapsed().as_secs_f64() * 1e6,
                segments,
            })
        }
    }
    fn draw(
        &mut self,
        icon: Icon,
        dc: HDC,
        bounds: RECT,
        size: i32,
        foreground: COLORREF,
        background: COLORREF,
    ) -> Result<()> {
        if self.target.is_none() {
            self.target = Some(Target::new(&self.factory)?);
        }
        let target = self
            .target
            .as_ref()
            .ok_or_else(|| invalid("No icon drawing target"))?;
        let x = bounds.left + (bounds.right - bounds.left - size) / 2;
        let y = bounds.top + (bounds.bottom - bounds.top - size) / 2;
        let rect = RECT {
            left: x,
            top: y,
            right: x + size,
            bottom: y + size,
        };
        // SAFETY: This paint DC and rectangle belong to the caller's live button.
        // Geometry, brush and DC target are retained, and no resource is recreated on hover.
        let result = unsafe {
            target.dc.BindDC(dc, &rect)?;
            target.dc.BeginDraw();
            target.dc.Clear(Some(&color(background)));
            let mut transform = Default::default();
            target.dc.GetTransform(&mut transform);
            transform.M11 = size as f32 / 20.0;
            transform.M22 = size as f32 / 20.0;
            transform.M12 = 0.0;
            transform.M21 = 0.0;
            transform.M31 = 0.0;
            transform.M32 = 0.0;
            target.dc.SetTransform(&transform);
            target.brush.SetColor(&color(foreground));
            target
                .dc
                .FillGeometry(&self.paths[icon as usize - 1], &target.brush, None);
            target.dc.EndDraw(None, None)
        };
        if result
            .as_ref()
            .is_err_and(|e| e.code() == windows::Win32::Foundation::D2DERR_RECREATE_TARGET)
        {
            self.target = None;
        }
        result
    }
}
impl Target {
    fn new(factory: &ID2D1Factory1) -> Result<Self> {
        // SAFETY: A small software DC target is independent of all video devices.
        unsafe {
            let dc = factory.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            })?;
            let brush = dc.CreateSolidColorBrush(&color(COLORREF(0)), None)?;
            Ok(Self { dc, brush })
        }
    }
}
fn color(c: COLORREF) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (c.0 & 255) as f32 / 255.0,
        g: ((c.0 >> 8) & 255) as f32 / 255.0,
        b: ((c.0 >> 16) & 255) as f32 / 255.0,
        a: 1.0,
    }
}
fn validate_source(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("Invalid embedded SVG encoding"))?;
    if !text.contains("viewBox=\"0 0 20 20\"")
        || text.matches("<path ").count() != 1
        || [
            "<g",
            "<image",
            "<use",
            "transform=",
            "fill-rule=",
            "stroke=",
            "<style",
            "href=",
            "<!",
        ]
        .iter()
        .any(|v| text.contains(v))
    {
        return Err(invalid(
            "Embedded Fluent SVG no longer matches the audited monochrome path format",
        ));
    }
    Ok(())
}
pub fn initialize() -> Result<()> {
    CACHE.with_borrow_mut(|slot| {
        if slot.is_none() {
            *slot = Some(Cache::new());
        }
        match slot.as_ref() {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(e.clone()),
            None => Err(invalid("Icon initialization failed")),
        }
    })
}
pub fn draw(
    icon: Icon,
    dc: HDC,
    bounds: RECT,
    size: i32,
    foreground: COLORREF,
    background: COLORREF,
) -> Result<()> {
    initialize()?;
    CACHE.with_borrow_mut(|slot| match slot.as_mut() {
        Some(Ok(cache)) => cache.draw(icon, dc, bounds, size, foreground, background),
        Some(Err(e)) => Err(e.clone()),
        None => Err(invalid("Icon cache unavailable")),
    })
}
fn private_bytes() -> Result<usize> {
    let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
    memory.cb = std::mem::size_of_val(&memory) as u32;
    // SAFETY: Current process pseudo-handle and fully sized writable counter structure.
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            memory.cb,
        )?;
    }
    Ok(memory.PrivateUsage)
}
pub fn benchmark() -> Result<()> {
    let before = private_bytes()?;
    let mut cache = Cache::new()?;
    let after = private_bytes()?;
    // SAFETY: Test owns the DC and bitmap, restores selection and destroys both
    // on every exit. Only its private in-memory surface is drawn; no UI input is injected.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.0.is_null() {
            return Err(Error::from_win32());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 40,
                biHeight: -40,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) => b,
            Err(e) => {
                let _ = DeleteDC(dc);
                return Err(e);
            }
        };
        let old = SelectObject(dc, bitmap.into());
        let result = (|| -> Result<()> {
            let icons = [
                Icon::Connect,
                Icon::Disconnect,
                Icon::Rotate,
                Icon::Control,
                Icon::Fullscreen,
                Icon::Restore,
                Icon::Settings,
                Icon::More,
            ];
            let mut cases = Vec::with_capacity(5);
            for size in [20, 25, 30, 35, 40] {
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: size,
                    bottom: size,
                };
                for icon in icons {
                    for (fg, bg) in [
                        (COLORREF(0xf5f5f5), COLORREF(0x202020)),
                        (COLORREF(0x1a1a1a), COLORREF(0xf9f9f9)),
                        (
                            COLORREF(GetSysColor(COLOR_WINDOWTEXT)),
                            COLORREF(GetSysColor(COLOR_WINDOW)),
                        ),
                    ] {
                        cache.draw(icon, dc, rect, size, fg, bg)?;
                        let mut ink = 0;
                        for y in 0..size {
                            for x in 0..size {
                                if GetPixel(dc, x, y) != bg {
                                    ink += 1;
                                }
                            }
                        }
                        if ink == 0 || ink >= size * size {
                            return Err(invalid("SVG paint has no visible bounded path"));
                        }
                    }
                }
                let mut times = Vec::with_capacity(1000);
                for n in 0..1050 {
                    let start = Instant::now();
                    cache.draw(
                        icons[n % 8],
                        dc,
                        rect,
                        size,
                        COLORREF(0xf5f5f5),
                        COLORREF(0x202020),
                    )?;
                    if n >= 50 {
                        times.push(start.elapsed().as_secs_f64() * 1e6);
                    }
                }
                times.sort_by(f64::total_cmp);
                cases.push(serde_json::json!({"dpi":size*96/20,"samples":times.len(),
                    "avg_us":times.iter().sum::<f64>()/times.len() as f64,
                    "p50_us":times[500],"p95_us":times[950],"p99_us":times[990]}));
            }
            let steady = private_bytes()?;
            println!(
                "{}",
                serde_json::json!({
                    "icon_count":SOURCES.len(), "svg_bytes":SOURCES.iter().map(|s|s.len()).sum::<usize>(),
                    "load_us":cache.load_us, "geometry_segments":cache.segments,
                    "private_bytes_before":before, "private_bytes_after":after,
                    "retained_private_delta_including_native_libraries":after as i64-before as i64,
                    "private_bytes_after_paint":steady,
                    "opaque_native_geometry_bytes":null,
                    "nonblank_palette_dpi_cases":120,
                    "fixed_dpi_repaint":cases,
                    "note":"Warm Direct2D icon-only CPU/DC paint; includes EndDraw. Not full UI repaint or physical input latency. Private delta includes OS initialization; COM allocation sizes are opaque."
                })
            );
            Ok(())
        })();
        SelectObject(dc, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_assets_match_supported_official_svg_format() {
        for source in SOURCES {
            assert!(validate_source(source).is_ok());
        }
    }
}
