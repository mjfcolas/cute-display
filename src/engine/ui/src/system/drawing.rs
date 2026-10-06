use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use core::ops::Range;

use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle, Triangle};

use super::ui_state::{Current, SettingRow, SystemUiState};
use crate::app_screen::DrawWithin;
use crate::mark::Mark;
use crate::text::{self, BODY, HINT, LIST, TITLE};

const SECTION_GAP: i32 = 6;
const COLUMN_GAP: i32 = 16;
const BAR_PADDING: i32 = 3;
const ROW_GAP: i32 = 4;
const BOX_PADDING: i32 = 3;
const CHOICE_GAP: i32 = 4;
const SETTING_GAP: i32 = 10;
const MORE_ARROW_WIDTH: i32 = 10;
const MORE_ARROW_HEIGHT: i32 = 6;

impl DrawWithin for SystemUiState {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let (left, top) = (area.top_left.x, area.top_left.y);
        let right = left + area.size.width as i32;
        let bottom = top + area.size.height as i32;
        let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
        text::write(target, self.title, area.top_left, area.size.width, &TITLE);
        let version = format!("Cute Display {}", self.version);
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

        let columns_bottom = hint_top - SECTION_GAP;
        let first_app_top = columns_top + heading(target, "apps", Point::new(left, columns_top), apps_width as u32, &HINT);
        let row_pitch = row_height() + ROW_GAP;
        let fitting = usize::try_from((columns_bottom - MORE_ARROW_HEIGHT - first_app_top + ROW_GAP) / row_pitch).unwrap_or(0);
        let chosen = self.apps.iter().position(|app| app.mark == Mark::Chosen);
        let shown = rows_in_sight(self.apps.len(), chosen, fitting);
        let mut apps_top = first_app_top;
        for app in self.apps.get(shown.clone()).unwrap_or_default() {
            apps_top += draw_row(target, app.title, Point::new(left, apps_top), apps_width as u32, app.mark) + ROW_GAP;
        }
        let more_left = left + apps_width - MORE_ARROW_WIDTH;
        if shown.start > 0 {
            let top = columns_top + (HINT.character_size.height as i32 - MORE_ARROW_HEIGHT) / 2;
            draw_more_arrow(target, Point::new(more_left, top + MORE_ARROW_HEIGHT), Point::new(more_left + MORE_ARROW_WIDTH / 2, top));
        }
        if shown.end < self.apps.len() {
            let top = columns_bottom - MORE_ARROW_HEIGHT;
            draw_more_arrow(target, Point::new(more_left, top), Point::new(more_left + MORE_ARROW_WIDTH / 2, top + MORE_ARROW_HEIGHT));
        }
        let mut settings_top = columns_top + heading(target, "settings", Point::new(settings_left, columns_top), settings_width, &HINT);
        for setting in &self.settings {
            settings_top += draw_setting(target, setting, Point::new(settings_left, settings_top), settings_width) + SETTING_GAP;
        }
        text::write(target, self.hint, Point::new(left, hint_top), area.size.width, &HINT);
    }
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

fn rows_in_sight(count: usize, chosen: Option<usize>, fitting: usize) -> Range<usize> {
    let start = chosen.map_or(0, |chosen| (chosen + 1).saturating_sub(fitting)).min(count.saturating_sub(fitting));
    start..(start + fitting).min(count)
}

fn draw_more_arrow<D: DrawTarget<Color = BinaryColor>>(target: &mut D, base_left: Point, tip: Point) {
    let base_right = Point::new(base_left.x + MORE_ARROW_WIDTH, base_left.y);
    let _ = Triangle::new(base_left, base_right, tip).into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
}

fn row_height() -> i32 {
    BODY.character_size.height as i32 + 2 * BAR_PADDING
}

fn draw_row<D: DrawTarget<Color = BinaryColor>>(target: &mut D, label: &str, top_left: Point, width: u32, mark: Mark) -> i32 {
    let height = row_height() as u32;
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
    use ui_testing::references;

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
            title: "System",
            version: "2026.9.0",
            apps: APPS.map(|title| AppRow { title, mark: mark(title) }).into(),
            settings: vec![
                SettingRow { name: SETTINGS[0], mark: mark(SETTINGS[0]), choices: choices(&["5 s", "10 s", "30 s", "always"]) },
                SettingRow { name: SETTINGS[1], mark: mark(SETTINGS[1]), choices: choices(&["off", "10%", "30%", "50%", "100%"]) },
            ],
            hint: "wheel: choose   click or long: open or change   yellow: back",
        }
    }

    fn render(state: &SystemUiState) -> Frame {
        let mut frame = Frame::blank();
        state.draw_within(&mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        for state in APPS.into_iter().chain(SETTINGS).map(state_choosing).chain(MANY_APPS.into_iter().map(state_with_many_apps_choosing)) {
            let chosen = state.apps.iter().find(|app| app.mark == Mark::Chosen).map_or("a setting", |app| app.title);
            let frame = render(&state);
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

    const MANY_APPS: [&str; 7] = ["Alarm clock", "Weather", "Radar", "Zen", "Timer", "Breathe", "Ephemeris"];

    fn state_with_many_apps_choosing(chosen: &str) -> SystemUiState {
        let apps = MANY_APPS.map(|title| AppRow { title, mark: if title == chosen { Mark::Chosen } else { Mark::Plain } }).into();
        SystemUiState { apps, ..state_choosing(chosen) }
    }

    #[test]
    fn the_rows_shown_move_down_only_as_far_as_the_chosen_one_needs() {
        assert_eq!(rows_in_sight(3, Some(2), 5), 0..3);
        assert_eq!(rows_in_sight(7, None, 5), 0..5);
        assert_eq!(rows_in_sight(7, Some(4), 5), 0..5);
        assert_eq!(rows_in_sight(7, Some(5), 5), 1..6);
        assert_eq!(rows_in_sight(7, Some(6), 5), 2..7);
        assert_eq!(rows_in_sight(7, Some(9), 5), 2..7);
        assert!(rows_in_sight(7, Some(3), 0).is_empty());
    }

    #[test]
    fn many_apps_stay_above_the_hint_whichever_is_chosen() {
        let hint_top = AREA.top_left.y + AREA.size.height as i32 - HINT.character_size.height as i32;
        for chosen in MANY_APPS.into_iter().chain(SETTINGS) {
            let frame = render(&state_with_many_apps_choosing(chosen));
            let gap = (hint_top - SECTION_GAP..hint_top).flat_map(|y| (AREA.top_left.x..AREA.top_left.x + 100).map(move |x| (x, y)));
            assert!(gap.into_iter().all(|(x, y)| !frame.is_ink(x, y)), "{chosen} chosen: an app runs into the hint");
            assert!(bar_lines_under_the_apps(&frame) > 0 || SETTINGS.contains(&chosen), "{chosen} chosen and out of sight");
        }
    }

    #[test]
    fn the_last_of_many_apps_chosen_looks_as_its_reference() {
        references::check(&render(&state_with_many_apps_choosing("Ephemeris")), env!("CARGO_MANIFEST_DIR"), "system-many-apps-scrolled");
    }

    #[test]
    fn many_apps_from_the_first_look_as_their_reference() {
        references::check(&render(&state_with_many_apps_choosing("Weather")), env!("CARGO_MANIFEST_DIR"), "system-many-apps");
    }

    #[test]
    fn an_app_chosen_looks_as_its_reference() {
        references::check(&render(&state_choosing("Radar")), env!("CARGO_MANIFEST_DIR"), "system-app-chosen");
    }

    #[test]
    fn a_setting_chosen_looks_as_its_reference() {
        references::check(&render(&state_choosing("Backlight")), env!("CARGO_MANIFEST_DIR"), "system-setting-chosen");
    }
}
