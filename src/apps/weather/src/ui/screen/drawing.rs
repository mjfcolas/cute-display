use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle};
use forecast::icons;
use forecast::Millimetres;
use ui::big_digits::TEMPERATURE;
use ui::text::{self, BODY, HINT, LIST, TITLE};

use super::ui_state::{HourColumn, ShownPage, TodayPage, WeatherUiState, WeekDay};
use super::HOURS;
use crate::ui::wind_arrow;

const GAP: i32 = 12;
const SECTION_GAP: i32 = 4;
const LINE_GAP: i32 = 2;
const PAGE_DOT: u32 = 8;
const TODAY_ICON: u32 = 64;
const HOUR_ICON: u32 = 20;
const WEEK_ICON: u32 = 20;
/// "Today" and a space.
const DAY_NAME_CHARS: i32 = 6;
/// Heavy rain: a full bar, and anything more too.
const FULL_BAR: Millimetres = Millimetres::from_tenths(40);
const BAR_MARGIN: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dot {
    Today,
    Week,
}

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(state: &WeatherUiState, target: &mut D, area: Rectangle) {
    let (left, top) = (area.top_left.x, area.top_left.y);
    let status_top = top + area.size.height as i32 - HINT.character_size.height as i32;
    text::write(target, &state.title, area.top_left, area.size.width, &TITLE);
    let shown = match state.page {
        ShownPage::Today(_) => Dot::Today,
        ShownPage::Week(_) => Dot::Week,
    };
    draw_page_dots(target, shown, Point::new(left + area.size.width as i32, top));

    let body_top = top + TITLE.character_size.height as i32 + SECTION_GAP;
    let body = Rectangle::new(Point::new(left, body_top), Size::new(area.size.width, (status_top - SECTION_GAP - body_top).max(0) as u32));
    match &state.page {
        ShownPage::Today(Some(today)) => draw_today(target, body, today),
        ShownPage::Week(Some(days)) => draw_week(target, body, days),
        ShownPage::Today(None) | ShownPage::Week(None) => text::write(target, "No forecast yet", body.top_left, body.size.width, &BODY),
    }
    text::write(target, &state.status, Point::new(left, status_top), area.size.width, &HINT);
}

fn draw_today<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, today: &TodayPage) {
    let (left, top) = (area.top_left.x, area.top_left.y);
    let right = left + area.size.width as i32;

    icons::draw(target, today.sky, area.top_left, TODAY_ICON);
    let degrees_left = left + TODAY_ICON as i32 + GAP;
    TEMPERATURE.draw_degrees(target, today.temperature.0, Point::new(degrees_left, top + (TODAY_ICON - TEMPERATURE.height) as i32 / 2));
    let lines_left = degrees_left + TEMPERATURE.degrees_width(today.temperature.0) as i32 + GAP;
    let lines_width = (right - lines_left).max(0) as u32;
    let mut line_top = top + (TODAY_ICON as i32 - lines_height()) / 2;
    text::write(target, today.sky_name, Point::new(lines_left, line_top), lines_width, &BODY);
    line_top += BODY.character_size.height as i32 + LINE_GAP;
    text::write(target, &today.feels_like, Point::new(lines_left, line_top), lines_width, &LIST);
    if let Some(day) = &today.day {
        line_top += LIST.character_size.height as i32 + LINE_GAP;
        text::write(target, day, Point::new(lines_left, line_top), lines_width, &LIST);
    }

    let details_top = top + TODAY_ICON as i32 + SECTION_GAP;
    let details = [("humidity", &today.humidity), ("pressure", &today.pressure), ("wind", &today.wind), ("sun", &today.sun)];
    let pitch = area.size.width as i32 / details.len() as i32;
    let value_top = details_top + HINT.character_size.height as i32 + LINE_GAP;
    for ((label, value), n) in details.iter().zip(0..) {
        let cell_left = left + n * pitch;
        text::write(target, label, Point::new(cell_left, details_top), pitch as u32, &HINT);
        text::write(target, value, Point::new(cell_left, value_top), pitch as u32, &LIST);
    }
    if let Some(from) = today.wind_from {
        let arrow_left = left + 2 * pitch + text::width(&today.wind, &LIST) as i32 + LINE_GAP * 2;
        wind_arrow::draw(target, from, Point::new(arrow_left, value_top), LIST.character_size.height);
    }

    let rule = details_top + (HINT.character_size.height + LIST.character_size.height) as i32 + LINE_GAP + SECTION_GAP;
    let _ = Line::new(Point::new(left, rule), Point::new(right - 1, rule))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(target);
    let hours_top = rule + SECTION_GAP;
    let bottom = top + area.size.height as i32;
    let hours = Rectangle::new(Point::new(left, hours_top), Size::new(area.size.width, (bottom - hours_top).max(0) as u32));
    draw_hours(target, hours, &today.hours);
}

fn lines_height() -> i32 {
    (BODY.character_size.height + 2 * LIST.character_size.height) as i32 + 2 * LINE_GAP
}

fn draw_hours<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, hours: &[HourColumn]) {
    let pitch = area.size.width as i32 / HOURS as i32;
    let first_centre = area.top_left.x + (area.size.width as i32 - pitch * HOURS as i32) / 2 + pitch / 2;
    let icon_top = area.top_left.y + HINT.character_size.height as i32 + LINE_GAP;
    let temperature_top = icon_top + HOUR_ICON as i32 + LINE_GAP;
    let rain_top = temperature_top + LIST.character_size.height as i32 + LINE_GAP;
    let baseline = area.top_left.y + area.size.height as i32 - 1;
    let tallest = baseline - (rain_top + HINT.character_size.height as i32 + LINE_GAP);
    if !hours.is_empty() {
        let last_centre = first_centre + (hours.len() as i32 - 1) * pitch;
        let _ = Line::new(Point::new(first_centre - pitch / 2, baseline), Point::new(last_centre + pitch / 2 - 1, baseline))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(target);
    }
    for (hour, n) in hours.iter().zip(0..) {
        let centre = first_centre + n * pitch;
        write_centred(target, &hour.hour, Point::new(centre, area.top_left.y), &HINT);
        icons::draw(target, hour.sky, Point::new(centre - HOUR_ICON as i32 / 2, icon_top), HOUR_ICON);
        write_centred(target, &hour.temperature, Point::new(centre, temperature_top), &LIST);
        if let Some(rain) = &hour.likely_rain {
            write_centred(target, rain, Point::new(centre, rain_top), &HINT);
        }
        let height = hour.fallen.map_or(0, |fallen| bar_height(fallen, tallest));
        let bar_width = (pitch - 2 * BAR_MARGIN).max(1) as u32;
        let bar = Rectangle::new(Point::new(centre - bar_width as i32 / 2, baseline - height), Size::new(bar_width, height as u32));
        let _ = bar.into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
    }
}

fn bar_height(fallen: Millimetres, tallest: i32) -> i32 {
    let tenths = i32::from(fallen.min(FULL_BAR).tenths());
    match tenths {
        0 => 0,
        _ => (tenths * tallest / i32::from(FULL_BAR.tenths())).max(1),
    }
}

fn draw_week<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, days: &[WeekDay]) {
    let pitch = (area.size.height as i32 / days.len().max(1) as i32).min(2 * BODY.character_size.height as i32);
    let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
    let icon_left = area.top_left.x + DAY_NAME_CHARS * advance;
    let range_left = icon_left + WEEK_ICON as i32 + GAP;
    let right = area.top_left.x + area.size.width as i32;
    let rain_offset = (BODY.character_size.height - LIST.character_size.height) as i32 / 2;
    for (day, n) in days.iter().zip(0..) {
        let top = area.top_left.y + n * pitch;
        text::write(target, day.name, Point::new(area.top_left.x, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
        icons::draw(target, day.sky, Point::new(icon_left, top), WEEK_ICON);
        text::write(target, &day.range, Point::new(range_left, top), (right - range_left).max(0) as u32, &BODY);
        if let Some(rain) = &day.rain {
            let width = text::width(rain, &LIST);
            text::write(target, rain, Point::new(right - width as i32, top + rain_offset), width, &LIST);
        }
    }
}

fn draw_page_dots<D: DrawTarget<Color = BinaryColor>>(target: &mut D, shown: Dot, top_right: Point) {
    for (dot, from_right) in [Dot::Week, Dot::Today].into_iter().zip(1..) {
        let circle = Circle::new(page_dot_top_left(top_right, from_right), PAGE_DOT);
        let style = if dot == shown { PrimitiveStyle::with_fill(BinaryColor::On) } else { PrimitiveStyle::with_stroke(BinaryColor::On, 1) };
        let _ = circle.into_styled(style).draw(target);
    }
}

fn page_dot_top_left(top_right: Point, from_right: i32) -> Point {
    let pitch = PAGE_DOT as i32 + GAP / 2;
    let top = top_right.y + (TITLE.character_size.height as i32 - PAGE_DOT as i32) / 2;
    Point::new(top_right.x - from_right * pitch + GAP / 2, top)
}

fn write_centred<D: DrawTarget<Color = BinaryColor>>(target: &mut D, line: &str, top_centre: Point, font: &MonoFont<'_>) {
    let width = text::width(line, font);
    text::write(target, line, Point::new(top_centre.x - width as i32 / 2, top_centre.y), width, font);
}

#[cfg(test)]
mod tests {
    use forecast::{CompassPoint, Degrees, Sky};
    use hal::display::{Frame, HEIGHT, WIDTH};
    use ui_testing::references;

    use super::*;

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn today_page() -> TodayPage {
        let hour = |n: u8| HourColumn {
            hour: format!("{:02}", 17 + n),
            sky: Sky::Storm,
            temperature: "-12°".into(),
            likely_rain: Some("100%".into()),
            fallen: Some(Millimetres::from_tenths(u16::from(n) * 5)),
        };
        TodayPage {
            sky: Sky::Snow,
            temperature: Degrees(-12),
            sky_name: "Partly cloudy",
            feels_like: "feels like -18°".into(),
            day: Some("-12° / 21°   rain 45%".into()),
            humidity: "100%".into(),
            pressure: "1016 hPa".into(),
            wind: "112 km/h".into(),
            wind_from: Some(CompassPoint::NorthWest),
            sun: "07:40-19:43".into(),
            hours: (0..HOURS as u8).map(hour).collect(),
        }
    }

    fn week() -> Vec<WeekDay> {
        ["Today", "Sat", "Sun", "Mon", "Tue", "Wed", "Thu"]
            .map(|name| WeekDay { name, sky: Sky::Rain, range: " -12° / 21°".into(), rain: Some("rain 100%".into()) })
            .into()
    }

    fn state(page: ShownPage) -> WeatherUiState {
        WeatherUiState { title: "Saint-Rémy-de-Provence".into(), page, status: "offline: no Wi-Fi: put cute-display/wifi.conf".into() }
    }

    fn render(state: &WeatherUiState) -> Frame {
        let mut frame = Frame::blank();
        draw(state, &mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let pages = [ShownPage::Today(None), ShownPage::Week(None), ShownPage::Today(Some(Box::new(today_page()))), ShownPage::Week(Some(week()))];
        for page in pages {
            let frame = render(&state(page.clone()));
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "{page:?}: ink at ({x},{y})");
                    }
                }
            }
        }
    }

    #[test]
    fn the_filled_dot_says_which_page_is_shown() {
        let top_right = Point::new(AREA.top_left.x + AREA.size.width as i32, AREA.top_left.y);
        let centre_inked = |frame: &Frame, from_right| {
            let centre = page_dot_top_left(top_right, from_right) + Point::new(PAGE_DOT as i32 / 2, PAGE_DOT as i32 / 2);
            frame.is_ink(centre.x, centre.y)
        };
        let (week, today) = (1, 2);
        let on_today = render(&state(ShownPage::Today(None)));
        assert!(centre_inked(&on_today, today) && !centre_inked(&on_today, week));
        let on_week = render(&state(ShownPage::Week(None)));
        assert!(centre_inked(&on_week, week) && !centre_inked(&on_week, today));
    }

    #[test]
    fn a_bar_grows_with_what_falls_up_to_heavy_rain() {
        let height = |tenths| bar_height(Millimetres::from_tenths(tenths), 20);
        assert_eq!(height(0), 0);
        assert_eq!(height(1), 1, "a drizzle still shows");
        assert_eq!(height(20), 10);
        assert_eq!(height(40), 20);
        assert_eq!(height(120), 20, "no higher than the room");
    }

    #[test]
    fn today_looks_as_its_reference() {
        references::check(&render(&state(ShownPage::Today(Some(Box::new(today_page()))))), env!("CARGO_MANIFEST_DIR"), "today");
    }

    #[test]
    fn the_week_looks_as_its_reference() {
        references::check(&render(&state(ShownPage::Week(Some(week())))), env!("CARGO_MANIFEST_DIR"), "week");
    }

    #[test]
    fn no_forecast_looks_as_its_reference() {
        references::check(&render(&state(ShownPage::Today(None))), env!("CARGO_MANIFEST_DIR"), "no-forecast");
    }
}
