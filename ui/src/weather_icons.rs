//! A picture for each sky, drawn from shapes at whatever size it is given, so the same
//! drawing serves today's large icon and the week's small ones.

use domain::weather::Sky;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle};

/// Draws `sky` inside the square at `top_left`, `side` pixels wide.
pub fn draw<D: DrawTarget<Color = BinaryColor>>(target: &mut D, sky: Sky, top_left: Point, side: u32) {
    let s = side as i32;
    let at = |x: i32, y: i32| top_left + Point::new(x * s / 100, y * s / 100);
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, (side / 16).max(1));
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);

    let sun = |target: &mut D, center: Point, diameter: u32| {
        let d = diameter as i32;
        let _ = Circle::with_center(center, diameter * 5 / 10).into_styled(fill).draw(target);
        for (dx, dy) in [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)] {
            let (inner, outer) = (d * 36 / 100, d * 48 / 100);
            let scale = |r: i32| if dx != 0 && dy != 0 { r * 7 / 10 } else { r };
            let from = center + Point::new(dx * scale(inner), dy * scale(inner));
            let to = center + Point::new(dx * scale(outer), dy * scale(outer));
            let _ = Line::new(from, to).into_styled(stroke).draw(target);
        }
    };
    // A cloud filling the box from `x0,y0` to `x1,y1`, in percent of the icon.
    let cloud = |target: &mut D, x0: i32, y0: i32, x1: i32, y1: i32| {
        let (w, h) = (x1 - x0, y1 - y0);
        let puff = |target: &mut D, cx: i32, cy: i32, d: i32| {
            let _ = Circle::with_center(at(x0 + w * cx / 100, y0 + h * cy / 100), (d * h * s / 10_000) as u32)
                .into_styled(fill)
                .draw(target);
        };
        puff(target, 30, 62, 70);
        puff(target, 55, 50, 90);
        puff(target, 76, 62, 70);
        let _ = Rectangle::with_corners(at(x0 + w * 30 / 100, y0 + h * 62 / 100), at(x0 + w * 76 / 100, y0 + h * 96 / 100))
            .into_styled(fill)
            .draw(target);
    };

    match sky {
        Sky::Clear => sun(target, at(50, 50), side),
        Sky::PartlyCloudy => {
            sun(target, at(36, 36), side * 7 / 10);
            cloud(target, 22, 40, 98, 94);
        }
        Sky::Cloudy => cloud(target, 4, 12, 96, 86),
        Sky::Fog => {
            for (y, x0, x1) in [(28, 10, 90), (46, 4, 80), (64, 18, 96), (82, 8, 86)] {
                let _ = Line::new(at(x0, y), at(x1, y)).into_styled(stroke).draw(target);
            }
        }
        Sky::Rain => {
            cloud(target, 4, 0, 96, 64);
            for x in [28, 50, 72] {
                let _ = Line::new(at(x, 72), at(x - 8, 96)).into_styled(stroke).draw(target);
            }
        }
        Sky::Snow => {
            cloud(target, 4, 0, 96, 64);
            for x in [26, 50, 74] {
                let _ = Circle::with_center(at(x, 84), (side / 10).max(2)).into_styled(fill).draw(target);
            }
        }
        Sky::Storm => {
            cloud(target, 4, 0, 96, 60);
            let _ = Triangle::new(at(56, 62), at(34, 84), at(52, 84)).into_styled(fill).draw(target);
            let _ = Triangle::new(at(52, 76), at(64, 76), at(42, 100)).into_styled(fill).draw(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use hal::display::{Frame, HEIGHT, WIDTH};

    use super::*;

    const SKIES: [Sky; 7] = [Sky::Clear, Sky::PartlyCloudy, Sky::Cloudy, Sky::Fog, Sky::Rain, Sky::Snow, Sky::Storm];

    fn icon(sky: Sky, side: u32) -> Frame {
        let mut frame = Frame::blank();
        draw(&mut frame, sky, Point::new(20, 20), side);
        frame
    }

    #[test]
    fn every_sky_stays_in_its_square_at_every_size() {
        for sky in SKIES {
            for side in [16, 20, 48, 96] {
                let frame = icon(sky, side);
                let mut inked = 0;
                for y in 0..i32::from(HEIGHT) {
                    for x in 0..i32::from(WIDTH) {
                        if frame.is_ink(x, y) {
                            inked += 1;
                            let inside = (20..20 + side as i32 + 1).contains(&x) && (20..20 + side as i32 + 1).contains(&y);
                            assert!(inside, "{sky:?} at {side} px inks ({x},{y})");
                        }
                    }
                }
                assert!(inked > 0, "{sky:?} at {side} px draws nothing");
            }
        }
    }

    #[test]
    fn no_two_skies_look_the_same() {
        for (n, a) in SKIES.iter().enumerate() {
            for b in SKIES.iter().skip(n + 1) {
                assert!(icon(*a, 20) != icon(*b, 20), "{a:?} and {b:?} look alike at 20 px");
            }
        }
    }
}
