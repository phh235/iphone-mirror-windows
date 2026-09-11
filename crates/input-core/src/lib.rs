mod pointer;
use imirror_coordinate_map::{Point, Size};
pub use pointer::{DEFAULT_POINTER_SPEED, MAX_POINTER_SPEED, MIN_POINTER_SPEED, PointerScale};
use std::time::Duration;
#[derive(Clone, Copy, Debug)]
pub enum Button {
    Home,
    Lock,
    VolumeUp,
    VolumeDown,
    AppSwitcher,
}
#[derive(Clone, Debug)]
pub enum Input {
    Tap(Point),
    Swipe {
        from: Point,
        to: Point,
        duration: Duration,
    },
    Text(String),
    Button(Button),
    Release,
}
pub trait Controller {
    type Error: std::error::Error + Send + Sync + 'static;
    fn geometry(&mut self) -> Result<Size, Self::Error>;
    fn dispatch(&mut self, input: Input) -> Result<(), Self::Error>;
}
/// Mouse gestures use mapped coordinates only. Leaving the viewport cancels a
/// gesture; Esc/focus loss always cancels, so another app cannot trigger a tap.
#[derive(Default)]
pub struct Gesture {
    down: Option<(Point, std::time::Instant)>,
}
impl Gesture {
    pub fn press(&mut self, point: Option<Point>, now: std::time::Instant) {
        self.down = point.map(|p| (p, now));
    }
    pub fn cancel(&mut self) {
        self.down = None;
    }
    pub fn release(&mut self, point: Option<Point>, now: std::time::Instant) -> Option<Input> {
        let (from, start) = self.down.take()?;
        let to = point?;
        let duration = now.saturating_duration_since(start);
        if (from.x - to.x).hypot(from.y - to.y) < 4.0 {
            Some(Input::Tap(to))
        } else {
            Some(Input::Swipe {
                from,
                to,
                duration: duration.clamp(Duration::from_millis(80), Duration::from_secs(5)),
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outside_and_cancel_never_tap() {
        let mut g = Gesture::default();
        let now = std::time::Instant::now();
        let p = Some(Point { x: 10.0, y: 10.0 });
        g.press(None, now);
        assert!(g.release(p, now).is_none());
        g.press(p, now);
        assert!(g.release(None, now).is_none());
        g.press(p, now);
        g.cancel();
        assert!(g.release(p, now).is_none());
    }
    #[test]
    fn drag_is_single_bounded_gesture() {
        let mut g = Gesture::default();
        let now = std::time::Instant::now();
        g.press(Some(Point { x: 10.0, y: 10.0 }), now);
        let result = g.release(
            Some(Point { x: 10.0, y: 100.0 }),
            now + Duration::from_secs(30),
        );
        assert!(
            matches!(result,Some(Input::Swipe{duration,..}) if duration==Duration::from_secs(5))
        );
        assert!(g.release(Some(Point { x: 10.0, y: 10.0 }), now).is_none());
    }
}
