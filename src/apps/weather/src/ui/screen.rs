use std::time::Instant;

use domain::clock::Clock;
use domain::fetch::FetchStatus;
use domain::time::TimeOfDay;
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle};
use forecast::icons;
use forecast::units::{percent, pressure, temperature, wind_speed};
use forecast::{DayForecast, Forecast, HourForecast, Millimetres, Sky, Weather, WeatherReport};
use ui::big_digits::TEMPERATURE;
use ui::calendar_names;
use ui::controls::{Button, Input};
use ui::text::{self, BODY, HINT, LIST, TITLE};
use ui::AppScreen;

use crate::ui::wind_arrow;

const GAP: i32 = 12;
const SECTION_GAP: i32 = 4;
const LINE_GAP: i32 = 2;
const PAGE_DOT: u32 = 8;
const TODAY_ICON: u32 = 64;
const HOURS: usize = 12;
const HOUR_ICON: u32 = 20;
const WEEK_ICON: u32 = 20;
/// "Today" and a space.
const DAY_NAME_CHARS: i32 = 6;
/// Below it, an hour's rain is not worth a number.
const LIKELY_RAIN: u8 = 10;
/// Heavy rain: a full bar, and anything more too.
const FULL_BAR: Millimetres = Millimetres::from_tenths(40);
const BAR_MARGIN: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Today,
    Week,
}

pub struct WeatherScreen {
    weather: Weather,
    clock: Clock,
    page: Page,
}

impl WeatherScreen {
    pub fn new(weather: Weather, clock: Clock) -> Self {
        Self { weather, clock, page: Page::Today }
    }

    fn today<'a>(&self, forecast: &'a Forecast) -> Option<&'a DayForecast> {
        self.clock.now().map_or(forecast.week.first(), |now| forecast.day(now.date))
    }

    fn coming_hours<'a>(&self, forecast: &'a Forecast) -> Vec<&'a HourForecast> {
        match self.clock.now() {
            Some(now) => forecast.hours_from(now).take(HOURS).collect(),
            None => forecast.hours.iter().take(HOURS).collect(),
        }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for WeatherScreen {
    /// The report's revision, and the minutes since it was fetched: "updated 12 min ago"
    /// has to move on by itself, and so do the hours.
    fn version(&self) -> u64 {
        let report = self.weather.report();
        (report.revision << 16) | (minutes_since(&report).unwrap_or(0) & 0xffff)
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) if detents > 0 => self.page = Page::Week,
            Input::Turn(detents) if detents < 0 => self.page = Page::Today,
            Input::Press(Button::Long) => self.weather.request_refresh(),
            Input::Turn(_) | Input::Press(Button::Yellow) | Input::HoldYellowAndLong => {}
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        let report = self.weather.report();
        let (left, top) = (area.top_left.x, area.top_left.y);
        let status_top = top + area.size.height as i32 - HINT.character_size.height as i32;
        text::write(target, report.place.as_deref().unwrap_or("Weather"), area.top_left, area.size.width, &TITLE);
        draw_page_dots(target, self.page, Point::new(left + area.size.width as i32, top));

        let body_top = top + TITLE.character_size.height as i32 + SECTION_GAP;
        let body = Rectangle::new(Point::new(left, body_top), Size::new(area.size.width, (status_top - SECTION_GAP - body_top).max(0) as u32));
        match (&report.forecast, self.page) {
            (Some(forecast), Page::Today) => self.draw_today(target, body, forecast),
            (Some(forecast), Page::Week) => draw_week(target, body, forecast),
            (None, _) => text::write(target, "No forecast yet", body.top_left, body.size.width, &BODY),
        }
        text::write(target, &status_line(&report), Point::new(left, status_top), area.size.width, &HINT);
    }
}

impl WeatherScreen {
    fn draw_today<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle, forecast: &Forecast) {
        let (left, top) = (area.top_left.x, area.top_left.y);
        let right = left + area.size.width as i32;
        let now = &forecast.today;
        let day = self.today(forecast);

        icons::draw(target, now.sky, area.top_left, TODAY_ICON);
        let degrees_left = left + TODAY_ICON as i32 + GAP;
        TEMPERATURE.draw_degrees(target, now.now.0, Point::new(degrees_left, top + (TODAY_ICON - TEMPERATURE.height) as i32 / 2));
        let lines_left = degrees_left + TEMPERATURE.degrees_width(now.now.0) as i32 + GAP;
        let lines_width = (right - lines_left).max(0) as u32;
        let mut line_top = top + (TODAY_ICON as i32 - lines_height()) / 2;
        text::write(target, sky_name(now.sky), Point::new(lines_left, line_top), lines_width, &BODY);
        line_top += BODY.character_size.height as i32 + LINE_GAP;
        text::write(target, &format!("feels like {}", temperature(now.feels_like)), Point::new(lines_left, line_top), lines_width, &LIST);
        if let Some(day) = day {
            line_top += LIST.character_size.height as i32 + LINE_GAP;
            text::write(target, &day_line(day), Point::new(lines_left, line_top), lines_width, &LIST);
        }

        let details_top = top + TODAY_ICON as i32 + SECTION_GAP;
        let sun = day.and_then(|day| Some(format!("{}-{}", clock_time(day.sunrise?), clock_time(day.sunset?))));
        let wind = wind_speed(now.wind.speed);
        let details = [
            ("humidity", percent(now.humidity)),
            ("pressure", pressure(now.pressure)),
            ("wind", wind.clone()),
            ("sun", sun.unwrap_or_else(|| "-".into())),
        ];
        let pitch = area.size.width as i32 / details.len() as i32;
        let value_top = details_top + HINT.character_size.height as i32 + LINE_GAP;
        for ((label, value), n) in details.iter().zip(0..) {
            let cell_left = left + n * pitch;
            text::write(target, label, Point::new(cell_left, details_top), pitch as u32, &HINT);
            text::write(target, value, Point::new(cell_left, value_top), pitch as u32, &LIST);
        }
        if now.wind.speed.0 > 0 {
            let arrow_left = left + 2 * pitch + text::width(&wind, &LIST) as i32 + LINE_GAP * 2;
            wind_arrow::draw(target, now.wind.from, Point::new(arrow_left, value_top), LIST.character_size.height);
        }

        let rule = details_top + (HINT.character_size.height + LIST.character_size.height) as i32 + LINE_GAP + SECTION_GAP;
        let _ = Line::new(Point::new(left, rule), Point::new(right - 1, rule))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(target);
        let hours_top = rule + SECTION_GAP;
        let bottom = top + area.size.height as i32;
        let hours = Rectangle::new(Point::new(left, hours_top), Size::new(area.size.width, (bottom - hours_top).max(0) as u32));
        draw_hours(target, hours, &self.coming_hours(forecast));
    }
}

fn lines_height() -> i32 {
    (BODY.character_size.height + 2 * LIST.character_size.height) as i32 + 2 * LINE_GAP
}

fn draw_hours<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, hours: &[&HourForecast]) {
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
        write_centred(target, &format!("{:02}", hour.start.time_of_day.hour()), Point::new(centre, area.top_left.y), &HINT);
        icons::draw(target, hour.sky, Point::new(centre - HOUR_ICON as i32 / 2, icon_top), HOUR_ICON);
        write_centred(target, &temperature(hour.temperature), Point::new(centre, temperature_top), &LIST);
        if let Some(rain) = hour.rain_chance.filter(|rain| rain.value() >= LIKELY_RAIN) {
            write_centred(target, &percent(rain), Point::new(centre, rain_top), &HINT);
        }
        let height = hour.precipitation.map_or(0, |fallen| bar_height(fallen, tallest));
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

fn draw_week<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, forecast: &Forecast) {
    let days = forecast.week.len().max(1) as i32;
    let pitch = (area.size.height as i32 / days).min(2 * BODY.character_size.height as i32);
    let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
    let icon_left = area.top_left.x + DAY_NAME_CHARS * advance;
    let range_left = icon_left + WEEK_ICON as i32 + GAP;
    let right = area.top_left.x + area.size.width as i32;
    let rain_offset = (BODY.character_size.height - LIST.character_size.height) as i32 / 2;
    for (day, n) in forecast.week.iter().zip(0..) {
        let top = area.top_left.y + n * pitch;
        let name = if n == 0 { "Today" } else { calendar_names::short_weekday(day.date.weekday()) };
        text::write(target, name, Point::new(area.top_left.x, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
        icons::draw(target, day.sky, Point::new(icon_left, top), WEEK_ICON);
        let range = format!("{:>4} / {}", temperature(day.low), temperature(day.high));
        text::write(target, &range, Point::new(range_left, top), (right - range_left).max(0) as u32, &BODY);
        if let Some(rain) = day.rain_chance {
            let rain = format!("rain {:>4}", percent(rain));
            let width = text::width(&rain, &LIST);
            text::write(target, &rain, Point::new(right - width as i32, top + rain_offset), width, &LIST);
        }
    }
}

fn draw_page_dots<D: DrawTarget<Color = BinaryColor>>(target: &mut D, shown: Page, top_right: Point) {
    let pitch = PAGE_DOT as i32 + GAP / 2;
    let top = top_right.y + (TITLE.character_size.height as i32 - PAGE_DOT as i32) / 2;
    for (page, from_right) in [Page::Week, Page::Today].into_iter().zip(1..) {
        let dot = Circle::new(Point::new(top_right.x - from_right * pitch + GAP / 2, top), PAGE_DOT);
        let style = if page == shown { PrimitiveStyle::with_fill(BinaryColor::On) } else { PrimitiveStyle::with_stroke(BinaryColor::On, 1) };
        let _ = dot.into_styled(style).draw(target);
    }
}

fn write_centred<D: DrawTarget<Color = BinaryColor>>(target: &mut D, line: &str, top_centre: Point, font: &MonoFont<'_>) {
    let width = text::width(line, font);
    text::write(target, line, Point::new(top_centre.x - width as i32 / 2, top_centre.y), width, font);
}

fn day_line(day: &DayForecast) -> String {
    let range = format!("{} / {}", temperature(day.low), temperature(day.high));
    match day.rain_chance {
        Some(rain) => format!("{range}   rain {}", percent(rain)),
        None => range,
    }
}

fn clock_time(time: TimeOfDay) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

fn minutes_since(report: &WeatherReport) -> Option<u64> {
    report.fetched_at.map(|at| Instant::now().saturating_duration_since(at).as_secs() / 60)
}

fn status_line(report: &WeatherReport) -> String {
    match &report.status {
        FetchStatus::NeverFetched => "waiting for the first update".into(),
        FetchStatus::Updating => "updating...".into(),
        FetchStatus::UpToDate => match minutes_since(report) {
            Some(0) | None => "updated just now   long: update   wheel: today/week".into(),
            Some(minutes) => format!("updated {minutes} min ago   long: update   wheel: today/week"),
        },
        FetchStatus::NoPlace => "no place: put cute-display/general.conf".into(),
        FetchStatus::Failed(why) => format!("offline: {why}"),
    }
}

fn sky_name(sky: Sky) -> &'static str {
    match sky {
        Sky::Clear => "Clear",
        Sky::PartlyCloudy => "Partly cloudy",
        Sky::Cloudy => "Cloudy",
        Sky::Fog => "Fog",
        Sky::Rain => "Rain",
        Sky::Snow => "Snow",
        Sky::Storm => "Storm",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::calendar::Date;
    use domain::clock::{TimeKeeper, TimeSource, TimeZoneSource};
    use domain::fetch::Unavailable;
    use domain::place::{GeoPoint, Place, PlaceSource};
    use domain::time::{LocalTime, UtcTime};
    use domain::time_zone::TimeZone;
    use forecast::{CompassPoint, Degrees, ForecastSource, Hectopascals, KilometresPerHour, Percent, Today, Wind};
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;

    struct Paris;

    impl PlaceSource for Paris {
        fn place(&mut self) -> Option<Place> {
            Some(Place { name: "Paris".into(), point: GeoPoint { latitude: 48.85, longitude: 2.35 } })
        }
    }

    #[derive(Clone, Default)]
    struct Answers(Arc<Mutex<Vec<Result<Forecast, Unavailable>>>>);

    impl ForecastSource for Answers {
        fn fetch(&mut self, _: &Place) -> Result<Forecast, Unavailable> {
            self.0.lock().unwrap().remove(0)
        }
    }

    struct StoppedAt(Option<UtcTime>);

    impl TimeKeeper for StoppedAt {
        fn read(&mut self) -> Option<UtcTime> {
            self.0
        }
        fn set(&mut self, _: UtcTime) {}
    }

    impl TimeSource for StoppedAt {
        fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
            Err(Unavailable("offline".into()))
        }
    }

    impl TimeZoneSource for StoppedAt {
        fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
            Ok(Some(TimeZone::UTC))
        }
    }

    fn friday() -> Date {
        Date::new(2026, 9, 25).unwrap()
    }

    fn clock_at(hour: Option<i64>) -> Clock {
        let time = hour.map(|hour| UtcTime::from_unix_seconds(friday().days_since_epoch() * 86_400 + hour * 3600 + 600));
        let clock = Clock::new(Box::new(StoppedAt(time)), Box::new(StoppedAt(None)), Box::new(StoppedAt(None)));
        clock.tick(Instant::now());
        clock
    }

    fn sample() -> Forecast {
        let skies = [Sky::Cloudy, Sky::Rain, Sky::PartlyCloudy, Sky::Clear, Sky::Fog, Sky::Snow, Sky::Storm];
        let week = skies
            .iter()
            .zip(0..)
            .map(|(&sky, n)| DayForecast {
                date: friday().plus_days(n),
                sky,
                low: Degrees(-12),
                high: Degrees(21),
                rain_chance: Percent::new(n as u8 * 15),
                sunrise: TimeOfDay::new(7, 40),
                sunset: TimeOfDay::new(19, 43),
            })
            .collect();
        let five = LocalTime { date: friday(), time_of_day: TimeOfDay::new(17, 0).unwrap(), second: 0 }.seconds_since_epoch();
        let hours = (0..24)
            .map(|n| HourForecast {
                start: LocalTime::from_seconds_since_epoch(five + n * 3600),
                sky: skies[n as usize % skies.len()],
                temperature: Degrees(n as i16 - 12),
                rain_chance: Percent::new(n as u8 * 4),
                precipitation: Some(Millimetres::from_tenths(n as u16 * 5)),
            })
            .collect();
        let today = Today {
            sky: Sky::Cloudy,
            now: Degrees(-12),
            feels_like: Degrees(-18),
            humidity: Percent::saturating(100),
            pressure: Hectopascals(1016),
            wind: Wind { speed: KilometresPerHour(112), from: CompassPoint::NorthWest },
        };
        Forecast { today, hours, week }
    }

    fn weather(answers: Vec<Result<Forecast, Unavailable>>) -> Weather {
        Weather::new(Box::new(Paris), Box::new(Answers(Arc::new(Mutex::new(answers)))))
    }

    fn fetched() -> Weather {
        let weather = weather(vec![Ok(sample())]);
        weather.refresh_if_due(Instant::now());
        weather
    }

    fn render(screen: &WeatherScreen) -> Frame {
        let mut frame = Frame::blank();
        let area = Rectangle::new(Point::new(8, 8), Size::new(u32::from(VISIBLE_WIDTH) - 16, u32::from(HEIGHT) - 16));
        AppScreen::<Frame>::draw(screen, &mut frame, area);
        frame
    }

    fn input(screen: &mut WeatherScreen, input: Input) {
        AppScreen::<Frame>::on_input(screen, input);
    }

    #[test]
    fn the_wheel_turns_to_the_week_and_back_to_today() {
        let mut screen = WeatherScreen::new(fetched(), clock_at(Some(18)));
        let today = render(&screen);
        input(&mut screen, Input::Turn(1));
        let week = render(&screen);
        assert!(week != today);
        input(&mut screen, Input::Turn(1));
        assert!(render(&screen) == week, "turning on stays on the week");
        input(&mut screen, Input::Turn(-1));
        assert!(render(&screen) == today);
    }

    #[test]
    fn the_hours_start_with_the_one_under_way() {
        let weather = fetched();
        let forecast = weather.report().forecast.unwrap();
        let first = |screen: &WeatherScreen| screen.coming_hours(&forecast).first().map(|hour| hour.start.time_of_day.hour());
        assert_eq!(first(&WeatherScreen::new(weather.clone(), clock_at(Some(20)))), Some(20));
        assert_eq!(first(&WeatherScreen::new(weather.clone(), clock_at(None))), Some(17), "the fetch's hours, the time unknown");
        assert_eq!(WeatherScreen::new(weather, clock_at(Some(20))).coming_hours(&forecast).len(), HOURS);
    }

    #[test]
    fn today_is_the_day_on_the_clock() {
        let weather = fetched();
        let forecast = weather.report().forecast.unwrap();
        let tomorrow_on_the_clock = WeatherScreen::new(weather.clone(), clock_at(Some(24 + 8)));
        assert_eq!(tomorrow_on_the_clock.today(&forecast).map(|day| day.date), Some(friday().plus_days(1)));
        assert_eq!(WeatherScreen::new(weather, clock_at(None)).today(&forecast).map(|day| day.date), Some(friday()));
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
    fn a_day_says_its_range_and_its_rain() {
        let mut day = sample().week[3].clone();
        assert_eq!(day_line(&day), "-12° / 21°   rain 45%");
        day.rain_chance = None;
        assert_eq!(day_line(&day), "-12° / 21°");
    }

    #[test]
    fn a_press_asks_for_an_update() {
        let weather = weather(vec![Ok(sample())]);
        let now = Instant::now();
        weather.refresh_if_due(now);
        let mut screen = WeatherScreen::new(weather.clone(), clock_at(None));
        assert!(!weather.is_due(now));
        input(&mut screen, Input::Press(Button::Long));
        assert!(weather.is_due(now));
    }

    #[test]
    fn the_version_moves_on_when_the_weather_changes() {
        let weather = weather(vec![Ok(sample())]);
        let screen = WeatherScreen::new(weather.clone(), clock_at(None));
        let before = AppScreen::<Frame>::version(&screen);
        weather.refresh_if_due(Instant::now());
        assert_ne!(AppScreen::<Frame>::version(&screen), before);
    }

    #[test]
    fn every_state_says_something_and_stays_on_the_glass() {
        let fetched = weather(vec![Ok(sample()), Err(Unavailable("no Wi-Fi: put cute-display/wifi.conf".into()))]);
        let mut frames = vec![render(&WeatherScreen::new(weather(vec![]), clock_at(None)))];
        fetched.refresh_if_due(Instant::now());
        let mut screen = WeatherScreen::new(fetched.clone(), clock_at(Some(17)));
        frames.push(render(&screen));
        input(&mut screen, Input::Turn(1));
        frames.push(render(&screen));
        fetched.request_refresh();
        fetched.refresh_if_due(Instant::now());
        frames.push(render(&screen));
        for frame in &frames {
            assert!(frame.as_bytes().iter().any(|&b| b != 0), "a blank screen");
            for x in i32::from(VISIBLE_WIDTH) - 8..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink in the margin at ({x},{y})");
                }
            }
            for x in 0..i32::from(VISIBLE_WIDTH) {
                for y in (0..8).chain(i32::from(HEIGHT) - 8..i32::from(HEIGHT)) {
                    assert!(!frame.is_ink(x, y), "ink in the margin at ({x},{y})");
                }
            }
        }
        assert_eq!(status_line(&fetched.report()), "offline: no Wi-Fi: put cute-display/wifi.conf");
    }
}
