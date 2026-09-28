//! Numbers in seven-segment digits, large enough to read across a dark room: the fonts
//! stop at 20 pixels.

use domain::time::TimeOfDay;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};

/// Segments a to g, a as bit 0: top, upper right, lower right, bottom, lower left,
/// upper left, middle.
const DIGITS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];
/// Only the middle bar.
const DASH: u8 = 0x40;

pub struct DigitSize {
    pub digit_width: u32,
    pub height: u32,
    stroke: u32,
    gap: u32,
}

pub const CLOCK: DigitSize = DigitSize { digit_width: 48, height: 86, stroke: 10, gap: 12 };
pub const TEMPERATURE: DigitSize = DigitSize { digit_width: 26, height: 48, stroke: 6, gap: 6 };

impl DigitSize {
    /// The colon, and a gap on each side.
    const fn colon_width(&self) -> u32 {
        self.stroke + 2 * self.gap
    }

    const fn degree_side(&self) -> u32 {
        self.digit_width / 2
    }

    /// How wide `hh:mm` is.
    pub const fn time_width(&self) -> u32 {
        4 * self.digit_width + 2 * self.gap + self.colon_width()
    }

    /// `hh:mm` from `top_left`; `None` draws dashes.
    pub fn draw_time<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, time: Option<TimeOfDay>, top_left: Point) {
        let digits = time.map_or([DASH; 4], |time| {
            let (hour, minute) = (time.hour(), time.minute());
            [segments(hour / 10), segments(hour % 10), segments(minute / 10), segments(minute % 10)]
        });
        let mut left = top_left.x;
        for (n, digit) in digits.into_iter().enumerate() {
            self.draw_digit(target, digit, Point::new(left, top_left.y));
            left += (self.digit_width + self.gap) as i32;
            if n == 1 {
                left -= self.gap as i32;
                self.draw_colon(target, Point::new(left + self.gap as i32, top_left.y));
                left += self.colon_width() as i32;
            }
        }
    }

    /// How wide `draw_degrees` draws `degrees`.
    pub fn degrees_width(&self, degrees: i16) -> u32 {
        let glyphs = degree_glyphs(degrees).len() as u32;
        glyphs * (self.digit_width + self.gap) + self.degree_side()
    }

    /// Whole degrees, `-12°`, from `top_left`.
    pub fn draw_degrees<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, degrees: i16, top_left: Point) {
        let mut left = top_left.x;
        for glyph in degree_glyphs(degrees) {
            self.draw_digit(target, glyph, Point::new(left, top_left.y));
            left += (self.digit_width + self.gap) as i32;
        }
        let ring = Rectangle::new(Point::new(left, top_left.y), Size::new(self.degree_side(), self.degree_side()));
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width((self.stroke / 2).max(1))
            .stroke_alignment(StrokeAlignment::Inside)
            .build();
        let _ = ring.into_styled(style).draw(target);
    }

    fn draw_digit<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, segments: u8, top_left: Point) {
        let (x, y) = (top_left.x, top_left.y);
        let (w, h, t) = (self.digit_width as i32, self.height as i32, self.stroke as i32);
        let middle = y + h / 2 - t / 2;
        let upright = (h - 3 * t) / 2;
        let bars = [
            (x + t, y, w - 2 * t, t),
            (x + w - t, y + t, t, upright),
            (x + w - t, middle + t, t, upright),
            (x + t, y + h - t, w - 2 * t, t),
            (x, middle + t, t, upright),
            (x, y + t, t, upright),
            (x + t, middle, w - 2 * t, t),
        ];
        for (n, (left, top, width, height)) in bars.into_iter().enumerate() {
            if segments & (1 << n) != 0 {
                fill(target, Rectangle::new(Point::new(left, top), Size::new(width as u32, height as u32)));
            }
        }
    }

    fn draw_colon<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, top_left: Point) {
        let dot = Size::new(self.stroke, self.stroke);
        let (third, half_stroke) = (self.height as i32 / 3, self.stroke as i32 / 2);
        fill(target, Rectangle::new(top_left + Point::new(0, third - half_stroke), dot));
        fill(target, Rectangle::new(top_left + Point::new(0, 2 * third - half_stroke), dot));
    }
}

fn segments(n: u8) -> u8 {
    DIGITS.get(usize::from(n)).copied().unwrap_or(DASH)
}

/// A dash below zero, then the digits.
fn degree_glyphs(degrees: i16) -> Vec<u8> {
    let sign = (degrees < 0).then_some(DASH);
    let digits = degrees.unsigned_abs().to_string().bytes().map(|b| segments(b.saturating_sub(b'0'))).collect::<Vec<_>>();
    sign.into_iter().chain(digits).collect()
}

fn fill<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle) {
    let _ = area.into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
}

#[cfg(test)]
mod tests {
    use hal::display::Frame;

    use super::*;

    fn ink(frame: &Frame, area: Rectangle) -> Vec<bool> {
        area.points().map(|p| frame.is_ink(p.x, p.y)).collect()
    }

    /// Ink at or right of `x`, or at or below `y`.
    fn ink_beyond(frame: &Frame, x: i32, y: i32) -> bool {
        (0..i32::from(hal::display::HEIGHT))
            .flat_map(|row| (0..i32::from(hal::display::WIDTH)).map(move |column| (column, row)))
            .any(|(column, row)| (column >= x || row >= y) && frame.is_ink(column, row))
    }

    #[test]
    fn every_digit_looks_different() {
        let second_digit = Rectangle::new(Point::new((CLOCK.digit_width + CLOCK.gap) as i32, 0), Size::new(CLOCK.digit_width, CLOCK.height));
        let mut seen = Vec::new();
        for hour in 0..10 {
            let mut frame = Frame::blank();
            CLOCK.draw_time(&mut frame, TimeOfDay::new(hour, 0), Point::zero());
            let digit = ink(&frame, second_digit);
            assert!(!seen.contains(&digit), "{hour} looks like another digit");
            seen.push(digit);
        }
    }

    #[test]
    fn the_time_stays_in_its_box() {
        let mut frame = Frame::blank();
        CLOCK.draw_time(&mut frame, TimeOfDay::new(23, 58), Point::zero());
        assert!(!ink_beyond(&frame, CLOCK.time_width() as i32, CLOCK.height as i32));
        assert!(frame.is_ink(CLOCK.time_width() as i32 - 1, CLOCK.height as i32 / 4), "the last digit reaches the right edge");
    }

    #[test]
    fn an_unknown_time_is_dashes() {
        let (mut known, mut unknown) = (Frame::blank(), Frame::blank());
        CLOCK.draw_time(&mut known, TimeOfDay::new(12, 34), Point::zero());
        CLOCK.draw_time(&mut unknown, None, Point::zero());
        assert!(known != unknown);
        assert!(!unknown.is_ink(CLOCK.digit_width as i32 / 2, 0), "no top bar on a dash");
        assert!(unknown.is_ink(CLOCK.digit_width as i32 / 2, CLOCK.height as i32 / 2), "a middle bar");
    }

    #[test]
    fn degrees_stay_in_their_box_below_zero_too() {
        for degrees in [7, 21, -3, -12, 104] {
            let mut frame = Frame::blank();
            TEMPERATURE.draw_degrees(&mut frame, degrees, Point::zero());
            let width = TEMPERATURE.degrees_width(degrees) as i32;
            assert!(!ink_beyond(&frame, width, TEMPERATURE.height as i32), "{degrees:?}");
            assert!(frame.is_ink(width - 1, 1), "{degrees:?}: the ring ends the box");
        }
        let (mut warm, mut cold) = (Frame::blank(), Frame::blank());
        TEMPERATURE.draw_degrees(&mut warm, 7, Point::zero());
        TEMPERATURE.draw_degrees(&mut cold, -7, Point::zero());
        assert!(cold.is_ink(TEMPERATURE.digit_width as i32 / 2, TEMPERATURE.height as i32 / 2), "a minus first");
        assert!(!warm.is_ink(TEMPERATURE.digit_width as i32 / 2, TEMPERATURE.height as i32 / 2));
    }
}
