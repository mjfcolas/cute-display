//! The wind as an arrow, pointing where the air goes: a wind from the south-west points
//! north-east, as on weather maps.

use domain::weather::CompassPoint;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Triangle};

/// Draws the arrow of a wind from `from` inside the square at `top_left`, `side` pixels
/// wide.
pub(crate) fn draw<D: DrawTarget<Color = BinaryColor>>(target: &mut D, from: CompassPoint, top_left: Point, side: u32) {
    let (dx, dy) = downwind(from);
    let half = side as i32 / 2;
    let centre = top_left + Point::new(half, half);
    // A diagonal runs corner to corner; its head is narrowed to look as wide as the others.
    let narrowed = |r: i32| if dx != 0 && dy != 0 { r * 7 / 10 } else { r };
    let along = |r: i32| Point::new(dx * r, dy * r);
    let across = |r: i32| Point::new(-dy * narrowed(r), dx * narrowed(r));
    let (tip, tail) = (centre + along(half - 1), centre - along(half - 1));
    let head_base = tip - along(half * 9 / 10);
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, (side / 8).max(1));
    let _ = Line::new(tail, head_base).into_styled(stroke).draw(target);
    let head = Triangle::new(tip, head_base + across(half * 6 / 10), head_base - across(half * 6 / 10));
    let _ = head.into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
}

/// A step towards where the air goes, y down the screen.
fn downwind(from: CompassPoint) -> (i32, i32) {
    match from {
        CompassPoint::North => (0, 1),
        CompassPoint::NorthEast => (-1, 1),
        CompassPoint::East => (-1, 0),
        CompassPoint::SouthEast => (-1, -1),
        CompassPoint::South => (0, -1),
        CompassPoint::SouthWest => (1, -1),
        CompassPoint::West => (1, 0),
        CompassPoint::NorthWest => (1, 1),
    }
}

#[cfg(test)]
mod tests {
    use hal::display::{Frame, HEIGHT, WIDTH};

    use super::*;

    const POINTS: [CompassPoint; 8] = [
        CompassPoint::North,
        CompassPoint::NorthEast,
        CompassPoint::East,
        CompassPoint::SouthEast,
        CompassPoint::South,
        CompassPoint::SouthWest,
        CompassPoint::West,
        CompassPoint::NorthWest,
    ];
    const SIDE: u32 = 13;

    fn arrow(from: CompassPoint) -> Frame {
        let mut frame = Frame::blank();
        draw(&mut frame, from, Point::new(20, 20), SIDE);
        frame
    }

    /// The sums of the inked pixels' x and y in the square's own coordinates, and their
    /// count; none may fall outside the square.
    fn ink_sums(frame: &Frame) -> (i32, i32, i32) {
        let mut sum = (0, 0, 0);
        for y in 0..i32::from(HEIGHT) {
            for x in 0..i32::from(WIDTH) {
                if frame.is_ink(x, y) {
                    let inside = (20..20 + SIDE as i32).contains(&x) && (20..20 + SIDE as i32).contains(&y);
                    assert!(inside, "ink at ({x},{y})");
                    sum = (sum.0 + x - 20, sum.1 + y - 20, sum.2 + 1);
                }
            }
        }
        sum
    }

    #[test]
    fn every_arrow_stays_in_its_square_and_differs() {
        let frames: Vec<Frame> = POINTS.iter().map(|&from| arrow(from)).collect();
        for (n, frame) in frames.iter().enumerate() {
            assert!(ink_sums(frame).2 > 0, "{:?} draws nothing", POINTS[n]);
            assert!(frames.iter().skip(n + 1).all(|other| other != frame), "{:?} looks like another", POINTS[n]);
        }
    }

    #[test]
    fn the_head_is_where_the_air_goes() {
        let centre = SIDE as i32 / 2;
        let (_, y, count) = ink_sums(&arrow(CompassPoint::South));
        assert!(y < centre * count, "a south wind points up the screen");
        let (x, _, count) = ink_sums(&arrow(CompassPoint::East));
        assert!(x < centre * count, "an east wind points left");
    }
}
