//! Linear relative-pointer gain. Integer residuals preserve slow movement.
pub const DEFAULT_POINTER_SPEED: u16 = 100;
pub const MIN_POINTER_SPEED: u16 = 5;
pub const MAX_POINTER_SPEED: u16 = 200;
#[derive(Debug)]
pub struct PointerScale {
    percent: u16,
    remainder: [i64; 2],
}
impl Default for PointerScale {
    fn default() -> Self {
        Self {
            percent: DEFAULT_POINTER_SPEED,
            remainder: [0, 0],
        }
    }
}
impl PointerScale {
    pub fn percent(&self) -> u16 {
        self.percent
    }
    pub fn set_percent(&mut self, percent: u16) {
        self.percent = percent.clamp(MIN_POINTER_SPEED, MAX_POINTER_SPEED);
        self.reset();
    }
    pub fn reset(&mut self) {
        self.remainder = [0, 0];
    }
    pub fn scale(&mut self, dx: i32, dy: i32) -> (i32, i32) {
        let x = Self::axis(dx, self.percent, &mut self.remainder[0]);
        let y = Self::axis(dy, self.percent, &mut self.remainder[1]);
        (x, y)
    }
    fn axis(delta: i32, percent: u16, remainder: &mut i64) -> i32 {
        let accumulated = i64::from(delta) * i64::from(percent) + *remainder;
        let raw = accumulated / 100;
        let output = raw.clamp(-32767, 32767);
        // Discard overflow rather than building a delayed trail of motion.
        *remainder = if output == raw { accumulated % 100 } else { 0 };
        output as i32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_moves_accumulate_without_drift() {
        let mut p = PointerScale::default();
        p.set_percent(25);
        let mut sum = (0, 0);
        for _ in 0..1000 {
            let (x, y) = p.scale(1, -1);
            sum.0 += x;
            sum.1 += y;
        }
        assert_eq!(sum, (250, -250));
        assert_eq!(p.scale(-1000, 1000), (-250, 250));
    }
    #[test]
    fn reversal_cancels_fractional_motion() {
        let mut p = PointerScale::default();
        p.set_percent(25);
        assert_eq!(p.scale(3, -3), (0, 0));
        assert_eq!(p.scale(-3, 3), (0, 0));
        assert_eq!(p.scale(4, -4), (1, -1));
    }
    #[test]
    fn capture_and_speed_changes_discard_residuals() {
        let mut p = PointerScale::default();
        p.set_percent(25);
        p.scale(3, 3);
        p.reset();
        assert_eq!(p.scale(1, 1), (0, 0));
        p.set_percent(50);
        assert_eq!(p.scale(1, 1), (0, 0));
        assert_eq!(p.scale(1, 1), (1, 1));
    }
    #[test]
    fn supported_speeds_and_large_jumps_are_bounded() {
        let mut p = PointerScale::default();
        p.set_percent(100);
        assert_eq!(p.scale(19, -21), (19, -21));
        p.set_percent(0);
        assert_eq!(p.percent(), 5);
        assert_eq!(p.scale(20, -20), (1, -1));
        p.set_percent(u16::MAX);
        assert_eq!(p.percent(), 200);
        assert_eq!(p.scale(i32::MAX, i32::MIN), (32767, -32767));
        assert_eq!(p.scale(0, 0), (0, 0));
    }
}
