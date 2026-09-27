//! Today's weather, or the week's, at the place in `cute-display/weather.conf`.

use std::time::Instant;

use domain::apps::App;
use domain::fetch::FetchStatus;
use domain::weather::{Forecast, Sky, Weather, WeatherReport};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::app_screen::AppScreen;
use crate::calendar_names;
use crate::degrees::temperature;
use crate::controls::{Button, Input};
use crate::text::{self, BODY, HINT};
use crate::weather_icons;

const TODAY_ICON: u32 = 88;
const WEEK_ICON: u32 = 16;
const WEEK_PITCH: i32 = 20;
const LINE_PITCH: i32 = 26;
const GAP: i32 = 16;
/// "Today" and a space.
const DAY_NAME_CHARS: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Today,
    Week,
}

pub struct WeatherScreen {
    weather: Weather,
    page: Page,
}

impl WeatherScreen {
    pub fn new(weather: Weather) -> Self {
        Self { weather, page: Page::Today }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for WeatherScreen {
    fn app(&self) -> App {
        App::Weather
    }

    /// The report's revision, and the minutes since it was fetched: "updated 12 min ago"
    /// has to move on by itself.
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
        let status_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
        let body = Rectangle::new(area.top_left, Size::new(area.size.width, (status_top - area.top_left.y - 4).max(0) as u32));

        match (&report.forecast, self.page) {
            (Some(forecast), Page::Today) => draw_today(target, body, report.place.as_deref().unwrap_or(""), forecast),
            (Some(forecast), Page::Week) => draw_week(target, body, forecast),
            (None, _) => text::write(target, "No forecast yet", body.top_left, body.size.width, &BODY),
        }
        text::write(target, &status_line(&report), Point::new(area.top_left.x, status_top), area.size.width, &HINT);
    }
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
        FetchStatus::NoPlace => "no place: put cute-display/weather.conf".into(),
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

fn draw_today<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, place: &str, forecast: &Forecast) {
    let today = &forecast.today;
    weather_icons::draw(target, today.sky, area.top_left, TODAY_ICON);
    let left = area.top_left.x + TODAY_ICON as i32 + GAP;
    let width = area.size.width.saturating_sub(TODAY_ICON + GAP as u32);
    let lines = [
        place.to_owned(),
        format!("{} now", temperature(today.now)),
        format!("{} / {}", temperature(today.low), temperature(today.high)),
        sky_name(today.sky).to_owned(),
    ];
    for (n, line) in lines.iter().enumerate() {
        text::write(target, line, Point::new(left, area.top_left.y + n as i32 * LINE_PITCH), width, &BODY);
    }
}

fn draw_week<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, forecast: &Forecast) {
    let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
    let icon_left = area.top_left.x + DAY_NAME_CHARS * advance;
    let text_left = icon_left + WEEK_ICON as i32 + GAP;
    let text_width = (area.top_left.x + area.size.width as i32 - text_left).max(0) as u32;
    for (n, day) in forecast.week.iter().enumerate() {
        let top = area.top_left.y + n as i32 * WEEK_PITCH;
        if top + WEEK_PITCH > area.top_left.y + area.size.height as i32 {
            break;
        }
        let name = if n == 0 { "Today" } else { calendar_names::short_weekday(day.date.weekday()) };
        text::write(target, name, Point::new(area.top_left.x, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
        weather_icons::draw(target, day.sky, Point::new(icon_left, top + 2), WEEK_ICON);
        let temperatures = format!("{:>4} / {}", temperature(day.low), temperature(day.high));
        text::write(target, &temperatures, Point::new(text_left, top), text_width, &BODY);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::calendar::Date;
    use domain::fetch::Unavailable;
    use domain::place::{GeoPoint, Place, PlaceSource};
    use domain::weather::{DayForecast, Degrees, ForecastSource, Today};
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

    fn sample() -> Forecast {
        let skies = [Sky::Cloudy, Sky::Rain, Sky::PartlyCloudy, Sky::Clear, Sky::Fog, Sky::Snow, Sky::Storm];
        let week = skies
            .iter()
            .enumerate()
            .map(|(n, &sky)| DayForecast { date: Date::new(2026, 9, 25).unwrap().plus_days(n as i64), sky, low: Degrees(8), high: Degrees(21) })
            .collect();
        Forecast { today: Today { sky: Sky::Cloudy, now: Degrees(19), low: Degrees(12), high: Degrees(21) }, hours: vec![], week }
    }

    fn weather(answers: Vec<Result<Forecast, Unavailable>>) -> Weather {
        Weather::new(Box::new(Paris), Box::new(Answers(Arc::new(Mutex::new(answers)))))
    }

    fn render(screen: &WeatherScreen) -> Frame {
        let mut frame = Frame::blank();
        let area = Rectangle::new(Point::new(12, 44), Size::new(u32::from(VISIBLE_WIDTH) - 24, 184));
        AppScreen::<Frame>::draw(screen, &mut frame, area);
        frame
    }

    fn input(screen: &mut WeatherScreen, input: Input) {
        AppScreen::<Frame>::on_input(screen, input);
    }

    #[test]
    fn the_wheel_turns_to_the_week_and_back_to_today() {
        let weather = weather(vec![Ok(sample())]);
        weather.refresh_if_due(Instant::now());
        let mut screen = WeatherScreen::new(weather);
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
    fn a_press_asks_for_an_update() {
        let weather = weather(vec![Ok(sample())]);
        let now = Instant::now();
        weather.refresh_if_due(now);
        let mut screen = WeatherScreen::new(weather.clone());
        assert!(!weather.is_due(now));
        input(&mut screen, Input::Press(Button::Long));
        assert!(weather.is_due(now));
    }

    #[test]
    fn the_version_moves_on_when_the_weather_changes() {
        let weather = weather(vec![Ok(sample())]);
        let screen = WeatherScreen::new(weather.clone());
        let before = AppScreen::<Frame>::version(&screen);
        weather.refresh_if_due(Instant::now());
        assert_ne!(AppScreen::<Frame>::version(&screen), before);
    }

    #[test]
    fn every_state_says_something_and_stays_on_the_glass() {
        let fetched = weather(vec![Ok(sample()), Err(Unavailable("no Wi-Fi: put cute-display/wifi.conf".into()))]);
        let mut frames = vec![render(&WeatherScreen::new(weather(vec![])))];
        fetched.refresh_if_due(Instant::now());
        let mut screen = WeatherScreen::new(fetched.clone());
        frames.push(render(&screen));
        input(&mut screen, Input::Turn(1));
        frames.push(render(&screen));
        fetched.request_refresh();
        fetched.refresh_if_due(Instant::now());
        frames.push(render(&screen));
        for frame in &frames {
            assert!(frame.as_bytes().iter().any(|&b| b != 0), "a blank screen");
            for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
                }
            }
        }
        assert_eq!(status_line(&fetched.report()), "offline: no Wi-Fi: put cute-display/wifi.conf");
    }
}
