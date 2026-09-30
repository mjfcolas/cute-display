use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use forecast::day_weather;
use ui::big_digits;
use ui::mark::Mark;
use ui::text::{self, BODY, HINT, TITLE};

use super::ui_state::{AlarmUiState, ClockPage, SettingsPage};

const ROW_PITCH: i32 = 22;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;
/// "Wednesday" and a space.
const DAY_NAME_CHARS: i32 = 10;
const GAP: i32 = 12;

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(state: &AlarmUiState, target: &mut D, area: Rectangle) {
    match state {
        AlarmUiState::Clock(page) => draw_clock(page, target, area),
        AlarmUiState::Settings(page) => draw_settings(page, target, area),
    }
}

fn draw_clock<D: DrawTarget<Color = BinaryColor>>(page: &ClockPage, target: &mut D, area: Rectangle) {
    let top = area.top_left.y;
    let right = area.top_left.x + area.size.width as i32;
    let digits_top = top + BODY.character_size.height as i32 + 2 * GAP;
    let mut date_width = area.size.width;
    if let Some(today) = &page.today {
        day_weather::draw_range(target, today, Point::new(right, top));
        date_width = date_width.saturating_sub(day_weather::range_width(today) + GAP as u32);
    }
    day_weather::draw_hours(target, page.hours.iter(), hours_column(area));
    text::write(target, &page.date, area.top_left, date_width, &BODY);
    big_digits::CLOCK.draw_time(target, page.time, Point::new(area.top_left.x, digits_top));

    let line_top = digits_top + big_digits::CLOCK.height as i32 + 2 * GAP;
    text::write(target, &page.alarm, Point::new(area.top_left.x, line_top), big_digits::CLOCK.time_width(), &BODY);
    write_hint(target, &page.hint, area);
}

fn draw_settings<D: DrawTarget<Color = BinaryColor>>(page: &SettingsPage, target: &mut D, area: Rectangle) {
    text::write(target, &page.title, area.top_left, area.size.width, &TITLE);
    let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
    let name_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
    let value_left = name_left + DAY_NAME_CHARS * advance;
    let value_width = (area.top_left.x + area.size.width as i32 - value_left).max(0) as u32;
    let first_top = area.top_left.y + TITLE.character_size.height as i32 + GAP;
    for (n, row) in (0..).zip(page.days.iter().chain([&page.ringtone])) {
        let top = first_top + n * ROW_PITCH;
        if row.mark == Mark::Chosen {
            let dot_top = top + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
            let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(target);
        }
        text::write(target, row.name, Point::new(name_left, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
        text::write(target, &row.value, Point::new(value_left, top), value_width, &BODY);
    }
    write_hint(target, page.hint, area);
}

fn hours_column(area: Rectangle) -> Rectangle {
    let right = area.top_left.x + area.size.width as i32;
    let room_beside_time = right - area.top_left.x - big_digits::CLOCK.time_width() as i32;
    let left = right - (room_beside_time + day_weather::HOURS_WIDTH as i32) / 2;
    let below_date = area.top_left.y + BODY.character_size.height as i32;
    let top = below_date + (hint_top(area) - below_date - day_weather::HOURS_HEIGHT as i32) / 2;
    Rectangle::new(Point::new(left, top), Size::new(day_weather::HOURS_WIDTH, day_weather::HOURS_HEIGHT))
}

fn hint_top(area: Rectangle) -> i32 {
    area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32
}

fn write_hint<D: DrawTarget<Color = BinaryColor>>(target: &mut D, hint: &str, area: Rectangle) {
    text::write(target, hint, Point::new(area.top_left.x, hint_top(area)), area.size.width, &HINT);
}

#[cfg(test)]
mod tests {
    use domain::calendar::Date;
    use domain::time::{LocalTime, TimeOfDay};
    use forecast::{DayForecast, Degrees, HourForecast, Millimetres, Percent, Sky};
    use hal::display::{Frame, HEIGHT, WIDTH};
    use ui_testing::references;

    use super::*;
    use crate::ui::screen::ui_state::SettingRow;

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));
    const CHOSEN_DAY: usize = 2;

    fn saturday() -> Date {
        Date::new(2026, 9, 26).unwrap()
    }

    fn clock_state(today: Option<DayForecast>, hours: Vec<HourForecast>) -> AlarmUiState {
        AlarmUiState::Clock(ClockPage {
            date: "Wednesday 30 September".into(),
            time: TimeOfDay::new(23, 59),
            today,
            hours,
            alarm: "Alarm tomorrow at 08:30".into(),
            hint: "long: snooze 9 min   hold yellow and long: stop".into(),
        })
    }

    fn clock_state_without_forecast() -> AlarmUiState {
        clock_state(None, vec![])
    }

    fn clock_state_with_forecast() -> AlarmUiState {
        let today = DayForecast {
            date: saturday(),
            sky: Sky::Rain,
            low: Degrees(-12),
            high: Degrees(14),
            rain_chance: Percent::new(90),
            sunrise: TimeOfDay::new(7, 41),
            sunset: TimeOfDay::new(19, 41),
        };
        let seven = LocalTime { date: saturday(), time_of_day: TimeOfDay::new(7, 0).unwrap(), second: 0 }.seconds_since_epoch();
        let hours = (0..day_weather::HOURS_SHOWN as i64)
            .map(|n| HourForecast {
                start: LocalTime::from_seconds_since_epoch(seven + n * 3600),
                sky: Sky::Storm,
                temperature: Degrees(-10),
                rain_chance: Percent::new(80),
                precipitation: Some(Millimetres::from_tenths(12)),
            })
            .collect();
        clock_state(Some(today), hours)
    }

    fn settings_state(value: &str) -> AlarmUiState {
        let row = |name, n| SettingRow { name, value: value.into(), mark: if n == CHOSEN_DAY { Mark::Chosen } else { Mark::Plain } };
        let names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
        AlarmUiState::Settings(Box::new(SettingsPage {
            title: "Alarm settings (alarm off)".into(),
            days: core::array::from_fn(|n| row(names[n], n)),
            ringtone: row("Ringtone", names.len()),
            hint: "wheel: ringtone, playing softly   long: done   yellow: cancel",
        }))
    }

    fn render(state: &AlarmUiState) -> Frame {
        let mut frame = Frame::blank();
        draw(state, &mut frame, AREA);
        frame
    }

    fn ink_in(frame: &Frame, area: Rectangle) -> bool {
        area.points().any(|point| frame.is_ink(point.x, point.y))
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let states = [clock_state_without_forecast(), clock_state_with_forecast(), settings_state("[07]:30"), settings_state("[Singing Bowl]")];
        for state in states {
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
    fn the_forecast_is_drawn_beside_the_time() {
        assert!(ink_in(&render(&clock_state_with_forecast()), hours_column(AREA)));
        assert!(!ink_in(&render(&clock_state_without_forecast()), hours_column(AREA)));
    }

    #[test]
    fn the_chosen_row_has_a_dot_beside_it() {
        let frame = render(&settings_state("07:30"));
        let first_top = AREA.top_left.y + TITLE.character_size.height as i32 + GAP;
        for n in 0..8 {
            let beside = Rectangle::new(Point::new(AREA.top_left.x, first_top + n * ROW_PITCH), Size::new(DOT_DIAMETER, BODY.character_size.height));
            assert_eq!(ink_in(&frame, beside), n == CHOSEN_DAY as i32, "row {n}");
        }
    }

    #[test]
    fn the_clock_looks_as_its_reference() {
        references::check(&render(&clock_state_with_forecast()), env!("CARGO_MANIFEST_DIR"), "clock");
    }

    #[test]
    fn the_settings_look_as_their_reference() {
        references::check(&render(&settings_state("[07]:30")), env!("CARGO_MANIFEST_DIR"), "settings");
    }
}
