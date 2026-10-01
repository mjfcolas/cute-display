use core::f32::consts::TAU;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Ellipse, Polyline, PrimitiveStyle, Rectangle};
use ui::big_digits;
use ui::mark::Mark;
use ui::text::{self, BODY, HINT, TITLE};
use ui::DrawWithin;

use super::ui_state::{SettingsPage, ZenPage, ZenUiState};

const PICTURE_WIDTH: u32 = 170;
const GAP: i32 = 12;
const WAVE_STROKE: u32 = 2;
const PAPER_BETWEEN_STONES: u32 = 2;
const MOON_DIAMETER: u32 = 30;
/// Bottom to top, each sitting on the one below.
const STONES: [Size; 3] = [Size::new(112, 36), Size::new(80, 28), Size::new(48, 22)];
const STONES_SINK: i32 = 4;
const WAVES: i32 = 3;
const WAVE_PITCH: i32 = 18;
const WAVE_LENGTH: f32 = 42.0;
const WAVE_HEIGHT: f32 = 4.0;
const WAVE_STEP: i32 = 3;
const SESSION_LINES: usize = 3;
const ROW_PITCH: i32 = 26;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;

impl DrawWithin for ZenUiState {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        match self {
            ZenUiState::Zen(page) => draw_zen(page, target, area),
            ZenUiState::Settings(page) => draw_settings(page, target, area),
        }
    }
}

fn draw_zen<D: DrawTarget<Color = BinaryColor>>(page: &ZenPage, target: &mut D, area: Rectangle) {
    let picture = Rectangle::new(area.top_left, Size::new(PICTURE_WIDTH, (hint_top(area) - GAP - area.top_left.y).max(0) as u32));
    draw_stones_on_waves(target, picture);

    let left = area.top_left.x + PICTURE_WIDTH as i32 + 2 * GAP;
    let width = (area.top_left.x + area.size.width as i32 - left).max(0) as u32;
    let mut top = area.top_left.y + GAP;
    big_digits::TEMPERATURE.draw_time(target, page.time, Point::new(left, top));
    top += big_digits::TEMPERATURE.height as i32 + 2 * GAP;
    text::write(target, &page.sound, Point::new(left, top), width, &BODY);
    top += BODY.character_size.height as i32 + GAP;
    for line in text::wrap(&page.session, text::chars_across(width, &BODY)).iter().take(SESSION_LINES) {
        text::write(target, line, Point::new(left, top), width, &BODY);
        top += BODY.character_size.height as i32;
    }
    if let Some(volume) = page.volume {
        text::write(target, &format!("Volume {}%", volume.as_percent()), Point::new(left, top), width, &BODY);
    }
    write_hint(target, page.hint, area);
}

/// Stones piled up in the water, its waves around them, the moon above.
fn draw_stones_on_waves<D: DrawTarget<Color = BinaryColor>>(target: &mut D, picture: Rectangle) {
    let ink = PrimitiveStyle::with_fill(BinaryColor::On);
    let paper = PrimitiveStyle::with_fill(BinaryColor::Off);
    let centre = picture.top_left.x + picture.size.width as i32 / 2;
    let water = picture.top_left.y + picture.size.height as i32 - WAVES * WAVE_PITCH;
    let mut bottom = water + WAVE_PITCH / 2;
    for size in STONES {
        let top_left = Point::new(centre - size.width as i32 / 2, bottom - size.height as i32);
        let parting = Size::new(size.width + 2 * PAPER_BETWEEN_STONES, size.height + 2 * PAPER_BETWEEN_STONES);
        let _ = Ellipse::new(top_left - Point::new(PAPER_BETWEEN_STONES as i32, PAPER_BETWEEN_STONES as i32), parting).into_styled(paper).draw(target);
        let _ = Ellipse::new(top_left, size).into_styled(ink).draw(target);
        bottom = top_left.y + STONES_SINK;
    }

    let moon = Point::new(picture.top_left.x + GAP, picture.top_left.y + GAP);
    let _ = Circle::new(moon, MOON_DIAMETER).into_styled(ink).draw(target);
    let _ = Circle::new(moon + Point::new(MOON_DIAMETER as i32 / 3, -(MOON_DIAMETER as i32) / 6), MOON_DIAMETER).into_styled(paper).draw(target);

    let right = picture.top_left.x + picture.size.width as i32;
    for wave in 0..WAVES {
        let level = water + wave * WAVE_PITCH + WAVE_PITCH / 2;
        let shift = wave as f32 * WAVE_LENGTH / 3.0;
        let points: Vec<Point> = (picture.top_left.x..=right)
            .step_by(WAVE_STEP as usize)
            .map(|x| Point::new(x, level + (((x as f32 + shift) / WAVE_LENGTH * TAU).sin() * WAVE_HEIGHT) as i32))
            .filter(|point| wave > 0 || (point.x - centre).unsigned_abs() > STONES[0].width / 2 + PAPER_BETWEEN_STONES)
            .collect();
        for piece in points.windows(2).filter(|pair| matches!(pair, [from, to] if to.x - from.x == WAVE_STEP)) {
            let _ = Polyline::new(piece).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, WAVE_STROKE)).draw(target);
        }
    }
}

fn draw_settings<D: DrawTarget<Color = BinaryColor>>(page: &SettingsPage, target: &mut D, area: Rectangle) {
    text::write(target, page.title, area.top_left, area.size.width, &TITLE);
    let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
    let name_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
    let name_chars = page.rows.iter().map(|row| row.name.chars().count()).max().unwrap_or(0) as i32 + 1;
    let value_left = name_left + name_chars * advance;
    let value_width = (area.top_left.x + area.size.width as i32 - value_left).max(0) as u32;
    let first_top = area.top_left.y + TITLE.character_size.height as i32 + 2 * GAP;
    for (n, row) in (0..).zip(page.rows.iter()) {
        let top = first_top + n * ROW_PITCH;
        if row.mark == Mark::Chosen {
            let dot_top = top + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
            let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(target);
        }
        text::write(target, row.name, Point::new(name_left, top), (name_chars * advance) as u32, &BODY);
        text::write(target, &row.value, Point::new(value_left, top), value_width, &BODY);
    }
    write_hint(target, page.hint, area);
}

fn hint_top(area: Rectangle) -> i32 {
    area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32
}

fn write_hint<D: DrawTarget<Color = BinaryColor>>(target: &mut D, hint: &str, area: Rectangle) {
    text::write(target, hint, Point::new(area.top_left.x, hint_top(area)), area.size.width, &HINT);
}

#[cfg(test)]
mod tests {
    use domain::time::TimeOfDay;
    use hal::display::{Frame, HEIGHT, WIDTH};
    use ui_testing::references;

    use domain::sound::Volume;

    use super::*;
    use crate::ui::screen::ui_state::SettingRow;

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn zen_state() -> ZenUiState {
        ZenUiState::Zen(ZenPage {
            time: TimeOfDay::new(22, 47),
            sound: "Norwegian fjell".into(),
            session: "7 h 13 min left".into(),
            volume: Some(Volume::percent(30)),
            hint: "wheel: volume   yellow: stop",
        })
    }

    fn settings_state() -> ZenUiState {
        let row = |name, value: &str, mark| SettingRow { name, value: value.into(), mark };
        ZenUiState::Settings(SettingsPage {
            title: "Zen settings",
            rows: [row("Start", "22:30", Mark::Plain), row("Length", "[8 h]", Mark::Chosen), row("Sound", "528hz calmness", Mark::Plain)],
            hint: "wheel: hours   long: done   yellow: cancel",
        })
    }

    fn render(state: &ZenUiState) -> Frame {
        let mut frame = Frame::blank();
        state.draw_within(&mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        for state in [zen_state(), settings_state()] {
            let frame = render(&state);
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "{state:?}: ink at ({x},{y})");
                    }
                }
            }
        }
    }

    #[test]
    fn the_zen_page_looks_as_its_reference() {
        references::check(&render(&zen_state()), env!("CARGO_MANIFEST_DIR"), "zen");
    }

    #[test]
    fn the_settings_look_as_their_reference() {
        references::check(&render(&settings_state()), env!("CARGO_MANIFEST_DIR"), "settings");
    }
}
