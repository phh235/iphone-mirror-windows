//! Phone-shaped window geometry. No video processing, controller or renderer state.
use crate::toolbar;
use windows::{
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
            MonitorFromWindow,
        },
        UI::{HiDpi::AdjustWindowRectExForDpi, WindowsAndMessaging::*},
    },
    core::{Error, Result},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent {
    pub width: i32,
    pub height: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Chrome {
    pub dpi: u32,
    pub borders: Extent,
    pub name: i32,
    pub status: i32,
    pub home: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Sizing {
    pub video: Extent,
    pub toolbar: i32,
    pub outer: Extent,
}
impl Chrome {
    pub fn px(self, value: i32) -> i32 {
        ((i64::from(value) * i64::from(self.dpi) + 48) / 96) as i32
    }
    pub fn toolbar(self, width: i32) -> i32 {
        toolbar::layout(width, self.dpi, self.name, self.status, self.home).height
    }
    /// Largest proportional video fitting a physical outer-window budget. The
    /// toolbar can reflow; evaluating its two heights avoids resize oscillation.
    pub fn fit(self, source: Extent, limit: Extent) -> Option<Sizing> {
        if source.width <= 0 || source.height <= 0 {
            return None;
        }
        let ratio = f64::from(source.width) / f64::from(source.height);
        let max_width = limit.width - self.borders.width;
        if max_width < 1 {
            return None;
        }
        let mut best: Option<Sizing> = None;
        for header in [self.px(44), self.px(54)] {
            let max_height = limit.height - self.borders.height - header;
            if max_height < 1 {
                continue;
            }
            let height = max_height
                .min((f64::from(max_width) / ratio).floor() as i32)
                .max(1);
            let width = (f64::from(height) * ratio).round() as i32;
            if width < 1 || width > max_width {
                continue;
            }
            let toolbar = self.toolbar(width);
            let candidate = Sizing {
                video: Extent { width, height },
                toolbar,
                outer: Extent {
                    width: width + self.borders.width,
                    height: height + toolbar + self.borders.height,
                },
            };
            if candidate.outer.height > limit.height {
                continue;
            }
            if best.is_none_or(|old| {
                i64::from(width) * i64::from(height)
                    > i64::from(old.video.width) * i64::from(old.video.height)
            }) {
                best = Some(candidate);
            }
        }
        best
    }
    pub fn natural(self, source: Extent, work: RECT) -> Option<Sizing> {
        let margin = self.px(16);
        self.fit(
            source,
            Extent {
                width: (work.right - work.left - 2 * margin).max(1),
                height: ((f64::from(work.bottom - work.top) * 0.88).floor() as i32)
                    .min(work.bottom - work.top - 2 * margin)
                    .max(1),
            },
        )
    }
    pub fn minimum(self, source: Extent, work: RECT) -> Option<Sizing> {
        self.fit(
            source,
            Extent {
                width: work.right - work.left,
                height: (self.px(160 + 54) + self.borders.height).min(work.bottom - work.top),
            },
        )
    }
    pub fn constrain(self, source: Extent, proposed: RECT, edge: u32, work: RECT) -> Option<RECT> {
        let minimum = self.minimum(source, work)?;
        let width_driven = matches!(edge, 1 | 2 | 4 | 5 | 7 | 8);
        let limit = Extent {
            width: if width_driven {
                (proposed.right - proposed.left)
                    .max(minimum.outer.width)
                    .min(work.right - work.left)
            } else {
                work.right - work.left
            },
            height: if width_driven {
                work.bottom - work.top
            } else {
                (proposed.bottom - proposed.top)
                    .max(minimum.outer.height)
                    .min(work.bottom - work.top)
            },
        };
        let size = self.fit(source, limit)?.outer;
        let left = if matches!(edge, 1 | 4 | 7) {
            proposed.right - size.width
        } else if matches!(edge, 3 | 6) {
            proposed.left + (proposed.right - proposed.left - size.width) / 2
        } else {
            proposed.left
        };
        let top = if matches!(edge, 3..=5) {
            proposed.bottom - size.height
        } else {
            proposed.top
        };
        Some(place(size, left, top, work))
    }
}
pub fn place(size: Extent, left: i32, top: i32, work: RECT) -> RECT {
    let left = left.clamp(work.left, (work.right - size.width).max(work.left));
    let top = top.clamp(work.top, (work.bottom - size.height).max(work.top));
    RECT {
        left,
        top,
        right: left + size.width,
        bottom: top + size.height,
    }
}
pub fn centered(size: Extent, work: RECT) -> RECT {
    place(
        size,
        work.left + (work.right - work.left - size.width) / 2,
        work.top + (work.bottom - work.top - size.height) / 2,
        work,
    )
}
pub fn borders(window: HWND, dpi: u32) -> Result<Extent> {
    let mut rect = RECT::default();
    // SAFETY: Read current styles from this live app window. A zero-sized client
    // rectangle yields only the DPI-correct non-client additions, including caption.
    unsafe {
        AdjustWindowRectExForDpi(
            &mut rect,
            WINDOW_STYLE(GetWindowLongW(window, GWL_STYLE) as u32),
            !GetMenu(window).0.is_null(),
            WINDOW_EX_STYLE(GetWindowLongW(window, GWL_EXSTYLE) as u32),
            dpi,
        )?;
    }
    Ok(Extent {
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}
pub fn monitor(window: HWND, suggested: Option<&RECT>) -> Result<MONITORINFO> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: Windows fills the correctly sized structure. For a DPI move, use
    // the suggested rectangle's target monitor, never the old monitor work area.
    unsafe {
        let handle = if let Some(rect) = suggested {
            MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST)
        } else {
            MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST)
        };
        if !GetMonitorInfoW(handle, &mut info).as_bool() {
            return Err(Error::from_win32());
        }
    }
    Ok(info)
}
pub fn side_padding(source: Extent, video: Extent) -> f64 {
    if source.width <= 0 || source.height <= 0 {
        return 0.0;
    }
    ((f64::from(video.width)
        - f64::from(video.height) * f64::from(source.width) / f64::from(source.height))
        / 2.0)
        .max(0.0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portrait_landscape_and_all_resize_edges_fit_at_five_dpis()
    -> std::result::Result<(), &'static str> {
        for dpi in [96, 120, 144, 168, 192] {
            let scale = |n| n * dpi as i32 / 96;
            let chrome = Chrome {
                dpi,
                borders: Extent {
                    width: scale(16),
                    height: scale(39),
                },
                name: scale(95),
                status: scale(110),
                home: false,
            };
            for source in [
                Extent {
                    width: 1180,
                    height: 2556,
                },
                Extent {
                    width: 2556,
                    height: 1180,
                },
            ] {
                for work in [
                    RECT {
                        left: 0,
                        top: 0,
                        right: 1920,
                        bottom: 1040,
                    },
                    RECT {
                        left: -1280,
                        top: 0,
                        right: 0,
                        bottom: 680,
                    },
                ] {
                    let result = chrome
                        .natural(source, work)
                        .ok_or("valid monitor fixture")?;
                    assert!(result.outer.width <= work.right - work.left);
                    assert!(result.outer.height <= work.bottom - work.top);
                    let expected = f64::from(result.video.height) * f64::from(source.width)
                        / f64::from(source.height);
                    assert!((f64::from(result.video.width) - expected).abs() <= 0.501);
                    assert!(side_padding(source, result.video) <= 0.251);
                    for edge in 1..=8 {
                        let proposed = RECT {
                            left: work.left + 35,
                            top: 20,
                            right: work.left + 450,
                            bottom: 610,
                        };
                        let rect = chrome
                            .constrain(source, proposed, edge, work)
                            .ok_or("resize fixture")?;
                        assert!(rect.left >= work.left && rect.right <= work.right);
                        assert!(rect.top >= work.top && rect.bottom <= work.bottom);
                        let width = rect.right - rect.left - chrome.borders.width;
                        let height =
                            rect.bottom - rect.top - chrome.borders.height - chrome.toolbar(width);
                        assert!(
                            (f64::from(width)
                                - f64::from(height) * f64::from(source.width)
                                    / f64::from(source.height))
                            .abs()
                                <= 0.501
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
