use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Polyline, PrimitiveStyle, Rectangle};
use ui::mark::Mark;
use ui::text::{self, BODY, HINT, LIST};
use ui::DrawWithin;

use super::ui_state::{BreathePage, Session};
use crate::domain::breathing::Fullness;

const PICTURE_WIDTH: u32 = 130;
const GAP: i32 = 12;
const BREATH_HEIGHT: u32 = 80;
const BREATH_STROKE: u32 = 3;
const EMPTY_LUNGS_STROKE: u32 = 1;
const STEPS_BELOW: i32 = 2;
const ROW_GAP: i32 = 6;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;

impl DrawWithin for BreathePage {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let hint_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
        let picture = Rectangle::new(area.top_left, Size::new(PICTURE_WIDTH, (hint_top - GAP - area.top_left.y).max(0) as u32));
        draw_breath(target, &self.breath, picture);

        let dot_left = area.top_left.x + PICTURE_WIDTH as i32 + 2 * GAP;
        let left = dot_left + DOT_DIAMETER as i32 + DOT_GAP;
        let width = (area.top_left.x + area.size.width as i32 - left).max(0) as u32;
        let (body_height, list_height) = (BODY.character_size.height as i32, LIST.character_size.height as i32);
        let mut top = area.top_left.y;
        for row in &self.rhythms {
            if row.mark == Mark::Chosen {
                let dot = Point::new(dot_left, top + (body_height - DOT_DIAMETER as i32) / 2);
                let _ = Circle::new(dot, DOT_DIAMETER).into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
            }
            text::write(target, row.name, Point::new(left, top), width, &BODY);
            text::write(target, &row.steps, Point::new(left, top + body_height + STEPS_BELOW), width, &LIST);
            top += body_height + STEPS_BELOW + list_height + ROW_GAP;
        }
        top += GAP;
        let status_width = (area.top_left.x + area.size.width as i32 - dot_left).max(0) as u32;
        text::write(target, &self.session.status(), Point::new(dot_left, top), status_width, &BODY);
        if let Session::Breathing { volume, .. } = self.session {
            text::write(target, &format!("Volume {}%", volume.as_percent()), Point::new(dot_left, top + body_height), status_width, &BODY);
        }
        text::write(target, self.hint, Point::new(area.top_left.x, hint_top), area.size.width, &HINT);
    }
}

fn draw_breath<D: DrawTarget<Color = BinaryColor>>(target: &mut D, breath: &[Fullness], picture: Rectangle) {
    let inset = BREATH_STROKE as i32;
    let (left, width) = (picture.top_left.x + inset, picture.size.width as i32 - 2 * inset);
    let bottom = picture.center().y + BREATH_HEIGHT as i32 / 2;
    let last = (breath.len() as i32 - 1).max(1);
    let points: Vec<Point> = (0..)
        .zip(breath)
        .map(|(n, fullness)| Point::new(left + n * width / last, bottom - i32::from(fullness.as_percent()) * BREATH_HEIGHT as i32 / 100))
        .collect();
    let empty_lungs = [Point::new(left, bottom), Point::new(left + width, bottom)];
    let _ = Polyline::new(&empty_lungs).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, EMPTY_LUNGS_STROKE)).draw(target);
    let _ = Polyline::new(&points).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, BREATH_STROKE)).draw(target);
}

#[cfg(test)]
mod tests {
    use domain::sound::Volume;
    use hal::display::{Frame, HEIGHT, WIDTH};
    use ui_testing::references;

    use super::*;
    use crate::domain::breathing::Rhythm;
    use crate::ui::screen::one_breath;
    use crate::ui::screen::ui_state::RhythmRow;

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn breathing_page() -> BreathePage {
        let row = |name, steps: &str, mark| RhythmRow { name, steps: steps.into(), mark };
        BreathePage {
            breath: one_breath(Rhythm::FourSevenEight),
            rhythms: [row("Coherence", "in 5, out 5 s", Mark::Plain), row("4-7-8", "in 4, hold 7, out 8 s", Mark::Chosen), row("Square", "in 4, hold 4, out 4, hold 4 s", Mark::Plain)],
            session: Session::Breathing { minutes: 12, volume: Volume::percent(30) },
            hint: "wheel: volume   yellow: stop",
        }
    }

    fn render(page: &BreathePage) -> Frame {
        let mut frame = Frame::blank();
        page.draw_within(&mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let long = BreathePage { session: Session::Breathing { minutes: 123_456_789_012, volume: Volume::percent(100) }, ..breathing_page() };
        for page in [breathing_page(), long] {
            let frame = render(&page);
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "ink at ({x},{y})");
                    }
                }
            }
        }
    }

    #[test]
    fn the_page_looks_as_its_reference() {
        references::check(&render(&breathing_page()), env!("CARGO_MANIFEST_DIR"), "breathing");
    }
}
