mod drawing;
mod ui_state;

use domain::clock::Clock;
use domain::fetch::FetchStatus;
use domain::time::TimeOfDay;
use forecast::units::{percent, pressure, temperature, wind_speed};
use forecast::{DayForecast, Forecast, HourForecast, Sky, Weather, WeatherReport};
use ui::calendar_names;
use ui::controls::{Button, Input};
use ui::Screen;

pub use ui_state::{HourColumn, ShownPage, TodayPage, WeatherUiState, WeekDay};

const HOURS: usize = 12;
/// Below it, an hour's rain is not worth a number.
const LIKELY_RAIN: u8 = 10;

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

    fn today_page(&self, forecast: &Forecast) -> TodayPage {
        let now = &forecast.today;
        let day = self.today(forecast);
        TodayPage {
            sky: now.sky,
            temperature: now.now,
            sky_name: sky_name(now.sky),
            feels_like: format!("feels like {}", temperature(now.feels_like)),
            day: day.map(day_line),
            humidity: percent(now.humidity),
            pressure: pressure(now.pressure),
            wind: wind_speed(now.wind.speed),
            wind_from: (now.wind.speed.0 > 0).then_some(now.wind.from),
            sun: day.and_then(|day| Some(format!("{}-{}", clock_time(day.sunrise?), clock_time(day.sunset?)))).unwrap_or_else(|| "-".into()),
            hours: self.coming_hours(forecast).into_iter().map(hour_column).collect(),
        }
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

impl Screen for WeatherScreen {
    type UiState = WeatherUiState;

    fn ui_state(&self) -> WeatherUiState {
        let report = self.weather.report();
        let forecast = report.forecast.as_ref();
        let page = match self.page {
            Page::Today => ShownPage::Today(forecast.map(|forecast| Box::new(self.today_page(forecast)))),
            Page::Week => ShownPage::Week(forecast.map(week)),
        };
        WeatherUiState {
            title: report.place.clone().unwrap_or_else(|| "Weather".into()),
            page,
            status: status_line(&report, &self.clock),
        }
    }

    /// The report's revision, and the minutes since it was fetched: "updated 12 min ago"
    /// has to move on by itself, and so do the hours.
    fn version(&self) -> u64 {
        let report = self.weather.report();
        (report.revision << 16) | (minutes_since(&report, &self.clock).unwrap_or(0) & 0xffff)
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) if detents > 0 => self.page = Page::Week,
            Input::Turn(detents) if detents < 0 => self.page = Page::Today,
            Input::Press(Button::Long) => self.weather.request_refresh(),
            Input::Turn(_) | Input::Press(Button::Yellow) | Input::HoldYellowAndLong => {}
        }
    }
}

fn hour_column(hour: &HourForecast) -> HourColumn {
    HourColumn {
        hour: format!("{:02}", hour.start.time_of_day.hour()),
        sky: hour.sky,
        temperature: temperature(hour.temperature),
        likely_rain: hour.rain_chance.filter(|rain| rain.value() >= LIKELY_RAIN).map(percent),
        fallen: hour.precipitation,
    }
}

fn week(forecast: &Forecast) -> Vec<WeekDay> {
    (0..)
        .zip(&forecast.week)
        .map(|(n, day)| WeekDay {
            name: if n == 0 { "Today" } else { calendar_names::short_weekday(day.date.weekday()) },
            sky: day.sky,
            range: format!("{:>4} / {}", temperature(day.low), temperature(day.high)),
            rain: day.rain_chance.map(|rain| format!("rain {:>4}", percent(rain))),
        })
        .collect()
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

fn minutes_since(report: &WeatherReport, clock: &Clock) -> Option<u64> {
    let (fetched, now) = (report.fetched_at?, clock.ticked_at()?);
    Some(now.saturating_duration_since(fetched).as_secs() / 60)
}

fn status_line(report: &WeatherReport, clock: &Clock) -> String {
    match &report.status {
        FetchStatus::NeverFetched => "waiting for the first update".into(),
        FetchStatus::Updating => "updating...".into(),
        FetchStatus::UpToDate => match minutes_since(report, clock) {
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
    use std::time::{Duration, Instant};

    use domain::calendar::Date;
    use domain::fetch::Unavailable;
    use domain::place::Place;
    use domain::time::{LocalTime, UtcTime};
    use domain_testing::place::{paris, StubPlace};
    use domain_testing::time::FakeTimeKeeper;
    use forecast::{CompassPoint, Degrees, ForecastSource, Hectopascals, KilometresPerHour, Millimetres, Percent, Today, Wind};

    use super::*;

    #[derive(Clone, Default)]
    struct StubForecastSource(Arc<Mutex<Vec<Result<Forecast, Unavailable>>>>);

    impl ForecastSource for StubForecastSource {
        fn fetch(&mut self, _: &Place) -> Result<Forecast, Unavailable> {
            self.0.lock().unwrap().remove(0)
        }
    }

    fn friday() -> Date {
        Date::new(2026, 9, 25).unwrap()
    }

    fn clock_at(hour: Option<i64>) -> Clock {
        let time = hour.map(|hour| UtcTime::from_unix_seconds(friday().days_since_epoch() * 86_400 + hour * 3600 + 600));
        let clock = time.map_or_else(FakeTimeKeeper::default, FakeTimeKeeper::at).offline_clock();
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
        Weather::new(Box::new(StubPlace::at(paris())), Box::new(StubForecastSource(Arc::new(Mutex::new(answers)))))
    }

    #[test]
    fn the_minutes_since_the_update_count_on_the_clocks_ticks() {
        let weather = weather(vec![Ok(sample())]);
        let fetched_at = Instant::now();
        weather.refresh_if_due(fetched_at);
        let clock = clock_at(Some(17));
        clock.tick(fetched_at + Duration::from_secs(12 * 60 + 30));
        assert!(WeatherScreen::new(weather.clone(), clock).ui_state().status.starts_with("updated 12 min ago"));
        let untold = FakeTimeKeeper::default().offline_clock();
        assert!(WeatherScreen::new(weather, untold).ui_state().status.starts_with("updated just now"));
    }

    fn fetched() -> Weather {
        let weather = weather(vec![Ok(sample())]);
        weather.refresh_if_due(Instant::now());
        weather
    }

    fn today_page(screen: &WeatherScreen) -> TodayPage {
        match screen.ui_state().page {
            ShownPage::Today(Some(page)) => *page,
            page => panic!("not today's forecast: {page:?}"),
        }
    }

    fn input(screen: &mut WeatherScreen, input: Input) {
        Screen::on_input(screen, input);
    }

    #[test]
    fn the_wheel_turns_to_the_week_and_back_to_today() {
        let mut screen = WeatherScreen::new(fetched(), clock_at(Some(18)));
        let page = |screen: &WeatherScreen| screen.ui_state().page;
        assert!(matches!(page(&screen), ShownPage::Today(Some(_))));
        input(&mut screen, Input::Turn(1));
        assert!(matches!(page(&screen), ShownPage::Week(Some(_))));
        input(&mut screen, Input::Turn(1));
        assert!(matches!(page(&screen), ShownPage::Week(Some(_))), "turning on stays on the week");
        input(&mut screen, Input::Turn(-1));
        assert!(matches!(page(&screen), ShownPage::Today(Some(_))));
    }

    #[test]
    fn the_hours_start_with_the_one_under_way() {
        let weather = fetched();
        let hours = |clock| today_page(&WeatherScreen::new(weather.clone(), clock)).hours;
        let first = |clock| hours(clock).first().map(|hour| hour.hour.clone());
        assert_eq!(first(clock_at(Some(20))).as_deref(), Some("20"));
        assert_eq!(first(clock_at(None)).as_deref(), Some("17"), "the fetch's hours, the time unknown");
        assert_eq!(hours(clock_at(Some(20))).len(), HOURS);
    }

    #[test]
    fn today_is_the_day_on_the_clock() {
        let weather = fetched();
        let day = |clock| today_page(&WeatherScreen::new(weather.clone(), clock)).day;
        assert_eq!(day(clock_at(Some(24 + 8))).as_deref(), Some("-12° / 21°   rain 15%"), "Saturday's");
        assert_eq!(day(clock_at(None)).as_deref(), Some("-12° / 21°   rain 0%"), "the first day's, the time unknown");
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
        let before = Screen::version(&screen);
        weather.refresh_if_due(Instant::now());
        assert_ne!(Screen::version(&screen), before);
    }

    #[test]
    fn before_any_forecast_either_page_says_there_is_none_yet() {
        let mut screen = WeatherScreen::new(weather(vec![]), clock_at(None));
        let state = screen.ui_state();
        assert_eq!((state.title.as_str(), state.page, state.status.as_str()), ("Weather", ShownPage::Today(None), "waiting for the first update"));
        input(&mut screen, Input::Turn(1));
        assert_eq!(screen.ui_state().page, ShownPage::Week(None));
    }

    #[test]
    fn today_says_the_sky_the_details_and_the_likely_rain_of_each_hour() {
        let page = today_page(&WeatherScreen::new(fetched(), clock_at(Some(17))));
        assert_eq!((page.sky_name, page.feels_like.as_str()), ("Cloudy", "feels like -18°"));
        assert_eq!([page.humidity, page.pressure, page.wind, page.sun], ["100%", "1016 hPa", "112 km/h", "07:40-19:43"]);
        assert_eq!(page.wind_from, Some(CompassPoint::NorthWest));
        let rain: Vec<Option<String>> = page.hours.iter().take(4).map(|hour| hour.likely_rain.clone()).collect();
        assert_eq!(rain, [None, None, None, Some("12%".into())], "below 10% it is not worth a number");
    }

    #[test]
    fn without_wind_there_is_no_arrow_and_without_the_sun_times_a_dash() {
        let mut calm = sample();
        calm.today.wind.speed = KilometresPerHour(0);
        for day in &mut calm.week {
            day.sunrise = None;
        }
        let weather = weather(vec![Ok(calm)]);
        weather.refresh_if_due(Instant::now());
        let page = today_page(&WeatherScreen::new(weather, clock_at(Some(17))));
        assert_eq!((page.wind_from, page.sun.as_str()), (None, "-"));
    }

    #[test]
    fn the_week_names_its_days_from_today() {
        let mut screen = WeatherScreen::new(fetched(), clock_at(Some(17)));
        input(&mut screen, Input::Turn(1));
        let ShownPage::Week(Some(days)) = screen.ui_state().page else { panic!("not the week") };
        let names: Vec<&str> = days.iter().map(|day| day.name).collect();
        assert_eq!(names, ["Today", "Sat", "Sun", "Mon", "Tue", "Wed", "Thu"]);
        assert_eq!((days[3].range.as_str(), days[3].rain.as_deref()), ("-12° / 21°", Some("rain  45%")));
    }

    #[test]
    fn offline_the_status_says_why() {
        let weather = weather(vec![Ok(sample()), Err(Unavailable("no Wi-Fi: put cute-display/wifi.conf".into()))]);
        weather.refresh_if_due(Instant::now());
        weather.request_refresh();
        weather.refresh_if_due(Instant::now());
        let status = WeatherScreen::new(weather, clock_at(Some(17))).ui_state().status;
        assert_eq!(status, "offline: no Wi-Fi: put cute-display/wifi.conf");
    }
}
