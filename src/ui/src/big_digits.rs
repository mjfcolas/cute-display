//! The time in seven-segment digits, large enough to read across a dark room: the
//! fonts stop at 20 pixels.

use domain::time::TimeOfDay;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

pub(crate) const DIGIT_WIDTH: u32 = 56;
pub(crate) const HEIGHT: u32 = 100;
const STROKE: u32 = 12;
const GAP: u32 = 14;
/// The colon, and a gap on each side.
const COLON_WIDTH: u32 = STROKE + 2 * GAP;

/// Segments a to g, a as bit 0: top, upper right, lower right, bottom, lower left,
/// upper left, middle.
const DIGITS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];
/// Only the middle bar.
const DASH: u8 = 0x40;

/// How wide `hh:mm` is.
pub(crate) const TIME_WIDTH: u32 = 4 * DIGIT_WIDTH + 2 * GAP + COLON_WIDTH;

/// `hh:mm` from `top_left`; `None` draws dashes.
pub(crate) fn draw_time<D: DrawTarget<Color = BinaryColor>>(target: &mut D, time: Option<TimeOfDay>, top_left: Point) {
    let segments = |n: u8| DIGITS.get(usize::from(n)).copied().unwrap_or(DASH);
    let digits = time.map_or([DASH; 4], |time| {
        let (hour, minute) = (time.hour(), time.minute());
        [segments(hour / 10), segments(hour % 10), segments(minute / 10), segments(minute % 10)]
    });
    let mut left = top_left.x;
    for (n, digit) in digits.into_iter().enumerate() {
        draw_digit(target, digit, Point::new(left, top_left.y));
        left += (DIGIT_WIDTH + GAP) as i32;
        if n == 1 {
            left -= GAP as i32;
            draw_colon(target, Point::new(left + GAP as i32, top_left.y));
            left += COLON_WIDTH as i32;
        }
    }
}

fn draw_digit<D: DrawTarget<Color = BinaryColor>>(target: &mut D, segments: u8, top_left: Point) {
    let (x, y) = (top_left.x, top_left.y);
    let (w, h, t) = (DIGIT_WIDTH as i32, HEIGHT as i32, STROKE as i32);
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

fn draw_colon<D: DrawTarget<Color = BinaryColor>>(target: &mut D, top_left: Point) {
    let dot = Size::new(STROKE, STROKE);
    fill(target, Rectangle::new(top_left + Point::new(0, HEIGHT as i32 / 3 - STROKE as i32 / 2), dot));
    fill(target, Rectangle::new(top_left + Point::new(0, 2 * HEIGHT as i32 / 3 - STROKE as i32 / 2), dot));
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

    #[test]
    fn every_digit_looks_different() {
        let second_digit = Rectangle::new(Point::new((DIGIT_WIDTH + GAP) as i32, 0), Size::new(DIGIT_WIDTH, HEIGHT));
        let mut seen = Vec::new();
        for hour in 0..10 {
            let mut frame = Frame::blank();
            draw_time(&mut frame, TimeOfDay::new(hour, 0), Point::zero());
            let digit = ink(&frame, second_digit);
            assert!(!seen.contains(&digit), "{hour} looks like another digit");
            seen.push(digit);
        }
    }

    #[test]
    fn the_time_stays_in_its_box() {
        let mut frame = Frame::blank();
        draw_time(&mut frame, TimeOfDay::new(23, 58), Point::zero());
        for x in 0..TIME_WIDTH as i32 + 8 {
            assert!(!frame.is_ink(x, HEIGHT as i32), "ink below at {x}");
        }
        for y in 0..HEIGHT as i32 {
            assert!(!frame.is_ink(TIME_WIDTH as i32, y), "ink right at {y}");
        }
        assert!(frame.is_ink(TIME_WIDTH as i32 - 1, HEIGHT as i32 / 4), "the last digit reaches the right edge");
    }

    #[test]
    fn an_unknown_time_is_dashes() {
        let (mut known, mut unknown) = (Frame::blank(), Frame::blank());
        draw_time(&mut known, TimeOfDay::new(12, 34), Point::zero());
        draw_time(&mut unknown, None, Point::zero());
        assert!(known != unknown);
        assert!(!unknown.is_ink(DIGIT_WIDTH as i32 / 2, 0), "no top bar on a dash");
        assert!(unknown.is_ink(DIGIT_WIDTH as i32 / 2, HEIGHT as i32 / 2), "a middle bar");
    }
}
