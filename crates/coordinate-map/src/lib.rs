//! Physical viewport to device coordinates, independent of transport/controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Rotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}
#[derive(Clone, Copy, Debug)]
pub enum ScaleMode {
    Fit,
    OneToOne,
}
#[derive(Debug, thiserror::Error)]
#[error("Viewport, stream and device dimensions must be finite and positive")]
pub struct MapError;
impl Size {
    fn valid(self) -> bool {
        self.width.is_finite() && self.height.is_finite() && self.width > 0.0 && self.height > 0.0
    }
}
impl Rect {
    pub fn contains(self, p: Point) -> bool {
        p.x.is_finite()
            && p.y.is_finite()
            && p.x >= self.origin.x
            && p.y >= self.origin.y
            && p.x < self.origin.x + self.size.width
            && p.y < self.origin.y + self.size.height
    }
    /// client_origin is in physical screen pixels, after the nonclient title bar.
    pub fn from_logical(self, scale: f64, client_origin: Point) -> Result<Self, MapError> {
        let result = Self {
            origin: Point {
                x: client_origin.x + self.origin.x * scale,
                y: client_origin.y + self.origin.y * scale,
            },
            size: Size {
                width: self.size.width * scale,
                height: self.size.height * scale,
            },
        };
        if !scale.is_finite()
            || scale <= 0.0
            || !result.size.valid()
            || !result.origin.x.is_finite()
            || !result.origin.y.is_finite()
        {
            return Err(MapError);
        }
        Ok(result)
    }
}
impl Rotation {
    pub fn from_quarters(value: u32) -> Self {
        match value % 4 {
            1 => Self::R90,
            2 => Self::R180,
            3 => Self::R270,
            _ => Self::R0,
        }
    }
    pub fn forward(self, p: Point) -> Point {
        match self {
            Self::R0 => p,
            Self::R90 => Point {
                x: 1.0 - p.y,
                y: p.x,
            },
            Self::R180 => Point {
                x: 1.0 - p.x,
                y: 1.0 - p.y,
            },
            Self::R270 => Point {
                x: p.y,
                y: 1.0 - p.x,
            },
        }
    }
    pub fn inverse(self, p: Point) -> Point {
        match self {
            Self::R90 => Self::R270.forward(p),
            Self::R270 => Self::R90.forward(p),
            _ => self.forward(p),
        }
    }
    fn size(self, s: Size) -> Size {
        if matches!(self, Self::R90 | Self::R270) {
            Size {
                width: s.height,
                height: s.width,
            }
        } else {
            s
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Mapper {
    viewport: Rect,
    image: Rect,
    device: Size,
    display_rotation: Rotation,
    device_rotation: Rotation,
}
impl Mapper {
    /// Device extent comes from the controller: WDA points are not video pixels.
    pub fn new(
        viewport: Rect,
        stream: Size,
        device: Size,
        display_rotation: Rotation,
        device_rotation: Rotation,
        mode: ScaleMode,
    ) -> Result<Self, MapError> {
        if !viewport.size.valid()
            || !stream.valid()
            || !device.valid()
            || !viewport.origin.x.is_finite()
            || !viewport.origin.y.is_finite()
        {
            return Err(MapError);
        }
        let displayed = display_rotation.size(stream);
        let scale = match mode {
            ScaleMode::Fit => {
                (viewport.size.width / displayed.width).min(viewport.size.height / displayed.height)
            }
            ScaleMode::OneToOne => 1.0,
        };
        let size = Size {
            width: displayed.width * scale,
            height: displayed.height * scale,
        };
        let image = Rect {
            origin: Point {
                x: viewport.origin.x + (viewport.size.width - size.width) / 2.0,
                y: viewport.origin.y + (viewport.size.height - size.height) / 2.0,
            },
            size,
        };
        Ok(Self {
            viewport,
            image,
            device,
            display_rotation,
            device_rotation,
        })
    }
    pub fn image_rect(self) -> Rect {
        self.image
    }
    pub fn map(self, p: Point) -> Option<Point> {
        if !self.viewport.contains(p) || !self.image.contains(p) {
            return None;
        }
        let uv = Point {
            x: (p.x - self.image.origin.x) / self.image.size.width,
            y: (p.y - self.image.origin.y) / self.image.size.height,
        };
        let uv = self
            .device_rotation
            .forward(self.display_rotation.inverse(uv));
        Some(Point {
            x: uv.x * self.device.width,
            y: uv.y * self.device.height,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
    fn mapper(rotation: Rotation, scale: f64, mode: ScaleMode) -> Result<Mapper, MapError> {
        let viewport = Rect {
            origin: Point { x: 0.0, y: 50.0 },
            size: Size {
                width: 600.0,
                height: 800.0,
            },
        }
        .from_logical(
            scale,
            Point {
                x: -1200.0,
                y: 100.0,
            },
        )?;
        Mapper::new(
            viewport,
            Size {
                width: 1080.0,
                height: 1920.0,
            },
            Size {
                width: 390.0,
                height: 844.0,
            },
            rotation,
            Rotation::R0,
            mode,
        )
    }
    #[test]
    fn dpi_offsets_letterboxing_all_rotations() -> Result<(), MapError> {
        for scale in [1.0, 1.25, 1.5, 2.0, 2.5] {
            for r in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
                let m = mapper(r, scale, ScaleMode::Fit)?;
                let p = Point { x: 0.2, y: 0.7 };
                let d = r.forward(p);
                let screen = Point {
                    x: m.image.origin.x + d.x * m.image.size.width,
                    y: m.image.origin.y + d.y * m.image.size.height,
                };
                let mapped = m.map(screen).ok_or(MapError)?;
                near(mapped.x, p.x * 390.0);
                near(mapped.y, p.y * 844.0);
                assert!(
                    m.map(Point {
                        x: m.image.origin.x - 0.01,
                        y: screen.y
                    })
                    .is_none()
                );
                assert!(
                    m.map(Point {
                        x: screen.x,
                        y: 100.0
                    })
                    .is_none()
                );
            }
        }
        Ok(())
    }
    #[test]
    fn one_to_one_clips_invisible_pixels() -> Result<(), MapError> {
        let m = mapper(Rotation::R0, 1.0, ScaleMode::OneToOne)?;
        near(m.image.size.width, 1080.0);
        assert!(m.map(m.image.origin).is_none());
        let p = m
            .map(Point {
                x: m.viewport.origin.x + 300.0,
                y: m.viewport.origin.y + 400.0,
            })
            .ok_or(MapError)?;
        near(p.x, 195.0);
        near(p.y, 422.0);
        Ok(())
    }
    #[test]
    fn direct_touch_uses_absolute_hid_units_across_dpi_and_rotation() -> Result<(), MapError> {
        for scale in [1.0, 1.5, 2.0] {
            let viewport = Rect {
                origin: Point { x: 12.0, y: 140.0 },
                size: Size {
                    width: 600.0,
                    height: 800.0,
                },
            }
            .from_logical(
                scale,
                Point {
                    x: -1000.0,
                    y: 50.0,
                },
            )?;
            for (rotation, u, v, x, y) in [
                (Rotation::R0, 0.2, 0.7, 2000.0, 7000.0),
                (Rotation::R90, 0.8, 0.25, 2500.0, 2000.0),
                (Rotation::R180, 0.1, 0.3, 9000.0, 7000.0),
                (Rotation::R270, 0.4, 0.9, 1000.0, 4000.0),
            ] {
                let mapper = Mapper::new(
                    viewport,
                    Size {
                        width: 1180.0,
                        height: 2556.0,
                    },
                    Size {
                        width: 10000.0,
                        height: 10000.0,
                    },
                    rotation,
                    Rotation::R0,
                    ScaleMode::Fit,
                )?;
                let image = mapper.image_rect();
                let point = mapper
                    .map(Point {
                        x: image.origin.x + u * image.size.width,
                        y: image.origin.y + v * image.size.height,
                    })
                    .ok_or(MapError)?;
                near(point.x, x);
                near(point.y, y);
                assert!(
                    mapper
                        .map(Point {
                            x: image.origin.x - 1.0,
                            y: image.origin.y + image.size.height * 0.5
                        })
                        .is_none()
                );
            }
        }
        Ok(())
    }
    #[test]
    fn invalid_geometry_rejected() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(mapper(Rotation::R0, bad, ScaleMode::Fit).is_err());
        }
    }
    #[test]
    fn independent_controller_rotation() -> Result<(), MapError> {
        let v = Rect {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 844.0,
                height: 390.0,
            },
        };
        let m = Mapper::new(
            v,
            v.size,
            Size {
                width: 390.0,
                height: 844.0,
            },
            Rotation::R0,
            Rotation::R90,
            ScaleMode::Fit,
        )?;
        let p = m.map(Point { x: 211.0, y: 78.0 }).ok_or(MapError)?;
        near(p.x, 312.0);
        near(p.y, 211.0);
        Ok(())
    }
    #[test]
    fn fullscreen_and_resize_preserve_center() -> Result<(), MapError> {
        for (w, h) in [
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (640.0, 1080.0),
            (1.0, 1.0),
        ] {
            let v = Rect {
                origin: Point { x: 0.0, y: 0.0 },
                size: Size {
                    width: w,
                    height: h,
                },
            };
            let m = Mapper::new(
                v,
                Size {
                    width: 1179.0,
                    height: 2556.0,
                },
                Size {
                    width: 393.0,
                    height: 852.0,
                },
                Rotation::R0,
                Rotation::R0,
                ScaleMode::Fit,
            )?;
            let p = m
                .map(Point {
                    x: w / 2.0,
                    y: h / 2.0,
                })
                .ok_or(MapError)?;
            near(p.x, 196.5);
            near(p.y, 426.0);
        }
        Ok(())
    }
}
