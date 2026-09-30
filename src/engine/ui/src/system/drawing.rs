use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};

use super::ui_state::{Current, SettingRow, SystemUiState};
use crate::mark::Mark;
use crate::text::{self, BODY, HINT, LIST, TITLE};

const NAME: &str = "System";
const HINTS: &str = "wheel: choose   click or long: open or change   yellow: back";
const SECTION_GAP: i32 = 6;
const COLUMN_GAP: i32 = 16;
const BAR_PADDING: i32 = 3;
const ROW_GAP: i32 = 4;
const BOX_PADDING: i32 = 3;
const CHOICE_GAP: i32 = 4;
const SETTING_GAP: i32 = 10;

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(state: &SystemUiState, target: &mut D, area: Rectangle) {
    let (left, top) = (area.top_left.x, area.top_left.y);
    let right = left + area.size.width as i32;
    let bottom = top + area.size.height as i32;
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    text::write(target, NAME, area.top_left, area.size.width, &TITLE);
    let version = format!("Cute Display {}", state.version);
    let version_width = text::width(&version, &HINT);
    let version_top = top + (TITLE.character_size.height - HINT.character_size.height) as i32;
    text::write(target, &version, Point::new(right - version_width as i32, version_top), version_width, &HINT);
    let rule = top + TITLE.character_size.height as i32 + SECTION_GAP;
    let _ = Line::new(Point::new(left, rule), Point::new(right - 1, rule)).into_styled(stroke).draw(target);

    let hint_top = bottom - HINT.character_size.height as i32;
    let columns_top = rule + 1 + SECTION_GAP;
    let apps_width = (area.size.width as i32 - COLUMN_GAP) * 2 / 5;
    let settings_left = left + apps_width + COLUMN_GAP;
    let settings_width = (right - settings_left).max(0) as u32;
    let divider = left + apps_width + COLUMN_GAP / 2;
    let _ = Line::new(Point::new(divider, columns_top), Point::new(divider, hint_top - SECTION_GAP)).into_styled(stroke).draw(target);

    let mut apps_top = columns_top + heading(target, "apps", Point::new(left, columns_top), apps_width as u32, &HINT);
    for app in &state.apps {
        apps_top += draw_row(target, app.title, Point::new(left, apps_top), apps_width as u32, app.mark) + ROW_GAP;
    }
    let mut settings_top = columns_top + heading(target, "settings", Point::new(settings_left, columns_top), settings_width, &HINT);
    for setting in &state.settings {
        settings_top += draw_setting(target, setting, Point::new(settings_left, settings_top), settings_width) + SETTING_GAP;
    }
    text::write(target, HINTS, Point::new(left, hint_top), area.size.width, &HINT);
}

fn draw_setting<D: DrawTarget<Color = BinaryColor>>(target: &mut D, setting: &SettingRow, top_left: Point, width: u32) -> i32 {
    let mut top = top_left.y + draw_row(target, setting.name, top_left, width, setting.mark);
    top += ROW_GAP;
    let right = top_left.x + width as i32;
    let mut left = top_left.x + BAR_PADDING - BOX_PADDING;
    for choice in &setting.choices {
        let size = Size::new(text::width(&choice.label, &LIST) + 2 * BOX_PADDING as u32, LIST.character_size.height + 2 * BOX_PADDING as u32);
        if left + size.width as i32 > right {
            break;
        }
        if choice.current == Current::Yes {
            let _ = Rectangle::new(Point::new(left, top), size).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1)).draw(target);
        }
        text::write(target, &choice.label, Point::new(left + BOX_PADDING, top + BOX_PADDING), size.width, &LIST);
        left += size.width as i32 + CHOICE_GAP;
    }
    top += LIST.character_size.height as i32 + 2 * BOX_PADDING;
    top - top_left.y
}

fn draw_row<D: DrawTarget<Color = BinaryColor>>(target: &mut D, label: &str, top_left: Point, width: u32, mark: Mark) -> i32 {
    let height = BODY.character_size.height + 2 * BAR_PADDING as u32;
    let color = match mark {
        Mark::Chosen => {
            let _ = Rectangle::new(top_left, Size::new(width, height)).into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
            BinaryColor::Off
        }
        Mark::Plain => BinaryColor::On,
    };
    let inner = width.saturating_sub(2 * BAR_PADDING as u32);
    text::write_in(target, label, top_left + Point::new(BAR_PADDING, BAR_PADDING), inner, &BODY, color);
    height as i32
}

fn heading<D: DrawTarget<Color = BinaryColor>>(target: &mut D, name: &str, top_left: Point, width: u32, font: &MonoFont<'_>) -> i32 {
    text::write(target, name, top_left + Point::new(BAR_PADDING, 0), width, font);
    font.character_size.height as i32 + ROW_GAP
}

#[cfg(test)]
mod tests {
    use hal::display::{Frame, HEIGHT, WIDTH};

    use super::*;
    use crate::system::ui_state::{AppRow, Choice};

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    const APPS: [&str; 3] = ["Alarm clock", "Weather", "Radar"];
    const SETTINGS: [&str; 2] = ["Backlight", "Reading lamp"];

    fn state_choosing(chosen: &str) -> SystemUiState {
        let mark = |name: &str| if name == chosen { Mark::Chosen } else { Mark::Plain };
        let choices = |labels: &[&str]| {
            let current = |n| if n == 1 { Current::Yes } else { Current::No };
            labels.iter().zip(0..).map(|(label, n)| Choice { label: (*label).into(), current: current(n) }).collect()
        };
        SystemUiState {
            version: "2026.9.0",
            apps: APPS.map(|title| AppRow { title, mark: mark(title) }).into(),
            settings: vec![
                SettingRow { name: SETTINGS[0], mark: mark(SETTINGS[0]), choices: choices(&["5 s", "10 s", "30 s", "always"]) },
                SettingRow { name: SETTINGS[1], mark: mark(SETTINGS[1]), choices: choices(&["off", "10%", "30%", "50%", "100%"]) },
            ],
        }
    }

    fn render(state: &SystemUiState) -> Frame {
        let mut frame = Frame::blank();
        draw(state, &mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        for chosen in APPS.into_iter().chain(SETTINGS) {
            let frame = render(&state_choosing(chosen));
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "{chosen} chosen: ink at ({x},{y})");
                    }
                }
            }
        }
    }

    fn bar_lines_under_the_apps(frame: &Frame) -> usize {
        let left = AREA.top_left.x;
        let right = AREA.top_left.x + AREA.size.width as i32 - 1;
        (AREA.top_left.y..AREA.top_left.y + AREA.size.height as i32)
            .filter(|&y| (left..left + 100).all(|x| frame.is_ink(x, y)) && !frame.is_ink(right, y))
            .count()
    }

    #[test]
    fn the_version_is_on_the_title_line_at_the_right() {
        let frame = render(&state_choosing("Alarm clock"));
        let right = AREA.top_left.x + AREA.size.width as i32;
        let title_line = AREA.top_left.y..AREA.top_left.y + TITLE.character_size.height as i32;
        assert!(title_line.clone().any(|y| (right - 20..right).any(|x| frame.is_ink(x, y))));
    }

    #[test]
    fn the_chosen_app_is_on_an_ink_bar() {
        assert!(bar_lines_under_the_apps(&render(&state_choosing("Radar"))) > 0);
        assert_eq!(bar_lines_under_the_apps(&render(&state_choosing("Backlight"))), 0);
    }
}
