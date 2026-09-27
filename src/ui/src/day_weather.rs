//! Today's weather in little room, for a screen that is about something else: the day's
//! sky and range on a line, a few hours in a column.

use domain::weather::{DayForecast, HourForecast};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::degrees::temperature;
use crate::text::{self, BODY, LIST};
use crate::weather_icons;

const RANGE_ICON: u32 = BODY.character_size.height;
const HOUR_ICON: u32 = 16;
const ICON_GAP: i32 = 4;
const HOUR_PITCH: i32 = 25;
const ADVANCE: i32 = (LIST.character_size.width + LIST.character_spacing) as i32;
/// "00:00".
const HOUR_LABEL_CHARS: i32 = 5;
/// "-10°".
const TEMPERATURE_CHARS: i32 = 4;
pub(crate) const HOURS_SHOWN: usize = 7;
pub(crate) const HOURS_WIDTH: u32 =
    ((HOUR_LABEL_CHARS + TEMPERATURE_CHARS) * ADVANCE + 2 * ICON_GAP) as u32 + HOUR_ICON;
pub(crate) const HOURS_HEIGHT: u32 = (HOURS_SHOWN as u32 - 1) * HOUR_PITCH as u32 + HOUR_ICON;

fn range(day: &DayForecast) -> String {
    format!("{} / {}", temperature(day.low), temperature(day.high))
}

/// How wide `draw_range` draws, for what shares its line.
pub(crate) fn range_width(day: &DayForecast) -> u32 {
    let advance = BODY.character_size.width + BODY.character_spacing;
    RANGE_ICON + ICON_GAP as u32 + range(day).chars().count() as u32 * advance
}

/// The day's sky and its low and high, in BODY, ending at `top_right`.
pub(crate) fn draw_range<D: DrawTarget<Color = BinaryColor>>(target: &mut D, day: &DayForecast, top_right: Point) {
    let left = top_right.x - range_width(day) as i32;
    weather_icons::draw(target, day.sky, Point::new(left, top_right.y), RANGE_ICON);
    let text_left = left + RANGE_ICON as i32 + ICON_GAP;
    text::write(target, &range(day), Point::new(text_left, top_right.y), (top_right.x - text_left).max(0) as u32, &BODY);
}

/// The first `HOURS_SHOWN` of `hours` down `area`, a row each: the hour, its sky, its
/// temperature.
pub(crate) fn draw_hours<'a, D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    hours: impl Iterator<Item = &'a HourForecast>,
    area: Rectangle,
) {
    let icon_left = area.top_left.x + HOUR_LABEL_CHARS * ADVANCE + ICON_GAP;
    let temperature_left = icon_left + HOUR_ICON as i32 + ICON_GAP;
    let temperature_width = (area.top_left.x + area.size.width as i32 - temperature_left).max(0) as u32;
    let text_top = (HOUR_ICON as i32 - LIST.character_size.height as i32) / 2;
    for (n, hour) in (0..).zip(hours.take(HOURS_SHOWN)) {
        let top = area.top_left.y + n * HOUR_PITCH;
        let label = format!("{:02}:00", hour.start.time_of_day.hour());
        text::write(target, &label, Point::new(area.top_left.x, top + text_top), (HOUR_LABEL_CHARS * ADVANCE) as u32, &LIST);
        weather_icons::draw(target, hour.sky, Point::new(icon_left, top), HOUR_ICON);
        text::write(target, &temperature(hour.temperature), Point::new(temperature_left, top + text_top), temperature_width, &LIST);
    }
}

#[cfg(test)]
mod tests {
    use domain::calendar::Date;
    use domain::time::{LocalTime, TimeOfDay};
    use domain::weather::{Degrees, Sky};
    use hal::display::Frame;

    use super::*;

    const AT: Point = Point::new(20, 10);

    /// Ink outside `area`, anywhere on the frame.
    fn ink_outside(frame: &Frame, area: Rectangle) -> Vec<Point> {
        let whole = Rectangle::new(Point::zero(), Size::new(hal::display::WIDTH.into(), hal::display::HEIGHT.into()));
        whole.points().filter(|&p| frame.is_ink(p.x, p.y) && !area.contains(p)).collect()
    }

    #[test]
    fn the_hours_stay_in_their_box() {
        let start = LocalTime { date: Date::new(2026, 9, 26).unwrap(), time_of_day: TimeOfDay::new(17, 0).unwrap(), second: 0 };
        let hours: Vec<HourForecast> = (0..HOURS_SHOWN as i64 + 2)
            .map(|n| HourForecast {
                start: LocalTime::from_seconds_since_epoch(start.seconds_since_epoch() + n * 3600),
                sky: Sky::Storm,
                temperature: Degrees(-10),
            })
            .collect();
        let area = Rectangle::new(AT, Size::new(HOURS_WIDTH, HOURS_HEIGHT));
        let mut frame = Frame::blank();
        draw_hours(&mut frame, hours.iter(), area);
        assert_eq!(ink_outside(&frame, area), []);
        let last_row = Rectangle::new(AT + Point::new(0, HOURS_HEIGHT as i32 - HOUR_ICON as i32), Size::new(HOURS_WIDTH, HOUR_ICON));
        assert!(last_row.points().any(|p| frame.is_ink(p.x, p.y)), "the last row is drawn");
    }

    #[test]
    fn the_range_ends_at_its_right_edge() {
        let day = DayForecast { date: Date::new(2026, 9, 26).unwrap(), sky: Sky::Rain, low: Degrees(-12), high: Degrees(-3) };
        let right = AT.x + range_width(&day) as i32;
        let mut frame = Frame::blank();
        draw_range(&mut frame, &day, Point::new(right, AT.y));
        let area = Rectangle::new(AT, Size::new(range_width(&day), BODY.character_size.height));
        assert_eq!(ink_outside(&frame, area), []);
        let last_character = Rectangle::new(Point::new(right - BODY.character_size.width as i32, AT.y), BODY.character_size);
        assert!(last_character.points().any(|p| frame.is_ink(p.x, p.y)), "the range reaches its right edge");
    }
}
