//! The weather at one place: today, the hours ahead and the days ahead. Fetched from
//! outside every hour, and on request; a failed fetch keeps the last forecast rather than
//! showing nothing.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use domain::apps::{AppService, Services};
use domain::calendar::Date;
use domain::fetch::{FetchStatus, Unavailable};
use domain::time::{LocalTime, TimeOfDay};
use place::{Place, PlaceFile, PlaceSource};

use crate::open_meteo::OpenMeteo;

pub mod day_weather;
pub mod icons;
pub mod open_meteo;
pub mod units;

/// Where the forecasts are for, in the app's files.
pub const PLACE_FILE: &str = "weather.conf";

pub const REFRESH_EVERY: Duration = Duration::from_secs(60 * 60);
pub const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);

/// What the sky is doing, in as many states as the device draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sky {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Rain,
    Snow,
    Storm,
}

/// Whole degrees Celsius.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Degrees(pub i16);

/// From 0 to 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percent(u8);

impl Percent {
    pub const fn new(percent: u8) -> Option<Self> {
        if percent <= 100 { Some(Self(percent)) } else { None }
    }

    pub fn saturating(percent: u8) -> Self {
        Self(percent.min(100))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

/// A depth of water, rain or melted snow, to a tenth of a millimetre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millimetres {
    tenths: u16,
}

impl Millimetres {
    pub const fn from_tenths(tenths: u16) -> Self {
        Self { tenths }
    }

    pub fn tenths(self) -> u16 {
        self.tenths
    }
}

/// Air pressure, brought down to sea level so that places compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hectopascals(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KilometresPerHour(pub u16);

/// One of eight points of the compass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompassPoint {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl CompassPoint {
    const CLOCKWISE: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    /// Clockwise from north.
    pub fn from_degrees(degrees: u16) -> Self {
        let point = (u32::from(degrees) * 2 + 45) / 90 % 8;
        Self::CLOCKWISE.get(point as usize).copied().unwrap_or(Self::North)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wind {
    pub speed: KilometresPerHour,
    /// Where it blows from.
    pub from: CompassPoint,
}

/// The weather at the moment of the fetch; the day's range is in the week.
#[derive(Clone, Debug, PartialEq)]
pub struct Today {
    pub sky: Sky,
    pub now: Degrees,
    pub feels_like: Degrees,
    pub humidity: Percent,
    pub pressure: Hectopascals,
    pub wind: Wind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayForecast {
    pub date: Date,
    pub sky: Sky,
    pub low: Degrees,
    pub high: Degrees,
    /// The likeliest hour's.
    pub rain_chance: Option<Percent>,
    /// None where the sun does not rise or set that day.
    pub sunrise: Option<TimeOfDay>,
    pub sunset: Option<TimeOfDay>,
}

/// The place's time zone is taken to be the clock's.
#[derive(Clone, Debug, PartialEq)]
pub struct HourForecast {
    pub start: LocalTime,
    pub sky: Sky,
    pub temperature: Degrees,
    pub rain_chance: Option<Percent>,
    /// What falls during the hour.
    pub precipitation: Option<Millimetres>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub today: Today,
    /// In order, from the hour of the fetch.
    pub hours: Vec<HourForecast>,
    /// Today first.
    pub week: Vec<DayForecast>,
}

impl Forecast {
    pub fn day(&self, date: Date) -> Option<&DayForecast> {
        self.week.iter().find(|day| day.date == date)
    }

    /// The hour `now` is in, and the ones after it.
    pub fn hours_from(&self, now: LocalTime) -> impl Iterator<Item = &HourForecast> {
        let this_hour = (now.date, now.time_of_day.hour());
        self.hours.iter().filter(move |hour| (hour.start.date, hour.start.time_of_day.hour()) >= this_hour)
    }
}

pub trait ForecastSource: Send {
    fn fetch(&mut self, place: &Place) -> Result<Forecast, Unavailable>;
}

/// Everything the weather is at one moment.
#[derive(Clone, Debug)]
pub struct WeatherReport {
    pub place: Option<String>,
    pub forecast: Option<Forecast>,
    /// When `forecast` was fetched.
    pub fetched_at: Option<Instant>,
    pub status: FetchStatus,
    /// Moves on at every change, so a screen knows when to redraw.
    pub revision: u64,
}

struct Sources {
    place: Box<dyn PlaceSource>,
    forecast: Box<dyn ForecastSource>,
}

struct State {
    report: WeatherReport,
    last_attempt: Option<Instant>,
    requested: bool,
}

/// Every clone is the same weather.
#[derive(Clone)]
pub struct Weather {
    state: Arc<Mutex<State>>,
    sources: Arc<Mutex<Sources>>,
}

impl Weather {
    pub fn new(place: Box<dyn PlaceSource>, forecast: Box<dyn ForecastSource>) -> Self {
        let report = WeatherReport { place: None, forecast: None, fetched_at: None, status: FetchStatus::NeverFetched, revision: 0 };
        Self {
            state: Arc::new(Mutex::new(State { report, last_attempt: None, requested: false })),
            sources: Arc::new(Mutex::new(Sources { place, forecast })),
        }
    }

    /// Open-Meteo's forecasts, at the place in the app's [`PLACE_FILE`].
    pub fn from_open_meteo(services: &dyn Services) -> Self {
        let place = PlaceFile::new(services.files(&[PLACE_FILE]), PLACE_FILE);
        Self::new(Box::new(place), Box::new(OpenMeteo::new(services.internet())))
    }

    pub fn report(&self) -> WeatherReport {
        self.lock_state().report.clone()
    }

    /// Fetch at the next chance, whenever the last fetch was.
    pub fn request_refresh(&self) {
        self.lock_state().requested = true;
    }

    pub fn is_due(&self, now: Instant) -> bool {
        let state = self.lock_state();
        let Some(last) = state.last_attempt else {
            return true;
        };
        let wait = match state.report.status {
            FetchStatus::Failed(_) | FetchStatus::NoPlace => RETRY_AFTER,
            _ => REFRESH_EVERY,
        };
        state.requested || now.saturating_duration_since(last) >= wait
    }

    /// Fetches if it is time to. Blocks for as long as the fetch takes, but never holds
    /// up whoever reads the report meanwhile.
    pub fn refresh_if_due(&self, now: Instant) {
        if !self.is_due(now) {
            return;
        }
        self.change(|state| {
            state.report.status = FetchStatus::Updating;
            state.requested = false;
            state.last_attempt = Some(now);
        });

        let mut sources = self.sources.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(place) = sources.place.place() else {
            drop(sources);
            self.change(|state| {
                state.report.place = None;
                state.report.status = FetchStatus::NoPlace;
            });
            return;
        };
        let fetched = sources.forecast.fetch(&place);
        drop(sources);

        self.change(|state| {
            state.report.place = Some(place.name);
            match fetched {
                Ok(forecast) => {
                    state.report.forecast = Some(forecast);
                    state.report.fetched_at = Some(now);
                    state.report.status = FetchStatus::UpToDate;
                }
                Err(unavailable) => state.report.status = FetchStatus::Failed(unavailable),
            }
        });
    }

    fn change(&self, change: impl FnOnce(&mut State)) {
        let mut state = self.lock_state();
        change(&mut state);
        state.report.revision = state.report.revision.wrapping_add(1);
    }

    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Fetched on the engine's network thread, whatever is on screen.
impl AppService for Weather {
    fn fetch_due(&self, now: Instant) -> bool {
        self.is_due(now)
    }

    fn fetch(&self, now: Instant) {
        self.refresh_if_due(now);
    }
}

#[cfg(test)]
mod tests {
    use domain::time::TimeOfDay;
    use place::GeoPoint;

    use super::*;

    struct Fixed(Option<Place>);

    impl PlaceSource for Fixed {
        fn place(&mut self) -> Option<Place> {
            self.0.clone()
        }
    }

    struct Script {
        answers: Vec<Result<Forecast, Unavailable>>,
        fetches: u32,
    }

    /// Answers with each result in turn, and counts the fetches.
    #[derive(Clone)]
    struct Scripted(Arc<Mutex<Script>>);

    impl ForecastSource for Scripted {
        fn fetch(&mut self, _: &Place) -> Result<Forecast, Unavailable> {
            let mut script = self.0.lock().unwrap();
            script.fetches += 1;
            script.answers.remove(0)
        }
    }

    impl Scripted {
        fn new(answers: Vec<Result<Forecast, Unavailable>>) -> Self {
            Self(Arc::new(Mutex::new(Script { answers, fetches: 0 })))
        }
        fn fetches(&self) -> u32 {
            self.0.lock().unwrap().fetches
        }
    }

    fn paris() -> Option<Place> {
        Some(Place { name: "Paris".into(), point: GeoPoint { latitude: 48.85, longitude: 2.35 } })
    }

    fn forecast(now: i16) -> Forecast {
        let today = Today {
            sky: Sky::Clear,
            now: Degrees(now),
            feels_like: Degrees(now),
            humidity: Percent(60),
            pressure: Hectopascals(1013),
            wind: Wind { speed: KilometresPerHour(10), from: CompassPoint::West },
        };
        Forecast { today, hours: vec![], week: vec![] }
    }

    fn weather(results: Vec<Result<Forecast, Unavailable>>) -> (Weather, Scripted) {
        let source = Scripted::new(results);
        (Weather::new(Box::new(Fixed(paris())), Box::new(source.clone())), source)
    }

    #[test]
    fn it_fetches_at_once_then_not_before_an_hour() {
        let (weather, source) = weather(vec![Ok(forecast(15)), Ok(forecast(16))]);
        let start = Instant::now();
        weather.refresh_if_due(start);
        weather.refresh_if_due(start + REFRESH_EVERY - Duration::from_secs(1));
        assert_eq!(source.fetches(), 1);
        weather.refresh_if_due(start + REFRESH_EVERY);
        assert_eq!(source.fetches(), 2);
        let report = weather.report();
        assert_eq!(report.status, FetchStatus::UpToDate);
        assert_eq!(report.place.as_deref(), Some("Paris"));
        assert_eq!(report.forecast.map(|f| f.today.now), Some(Degrees(16)));
    }

    #[test]
    fn a_failure_keeps_the_last_forecast_and_retries_sooner() {
        let (weather, source) = weather(vec![Ok(forecast(15)), Err(Unavailable("no Wi-Fi".into())), Ok(forecast(17))]);
        let start = Instant::now();
        weather.refresh_if_due(start);
        weather.request_refresh();
        weather.refresh_if_due(start);
        let report = weather.report();
        assert_eq!(report.status, FetchStatus::Failed(Unavailable("no Wi-Fi".into())));
        assert_eq!(report.forecast.map(|f| f.today.now), Some(Degrees(15)));

        weather.refresh_if_due(start + RETRY_AFTER);
        assert_eq!(source.fetches(), 3);
    }

    #[test]
    fn a_request_is_served_at_the_next_chance() {
        let (weather, source) = weather(vec![Ok(forecast(15)), Ok(forecast(16))]);
        let start = Instant::now();
        weather.refresh_if_due(start);
        weather.request_refresh();
        weather.refresh_if_due(start + Duration::from_secs(1));
        assert_eq!(source.fetches(), 2);
    }

    #[test]
    fn a_missing_place_is_said_and_nothing_is_fetched() {
        let source = Scripted::new(vec![]);
        let weather = Weather::new(Box::new(Fixed(None)), Box::new(source.clone()));
        weather.refresh_if_due(Instant::now());
        assert_eq!(weather.report().status, FetchStatus::NoPlace);
        assert_eq!(source.fetches(), 0);
    }

    #[test]
    fn every_change_moves_the_revision_on() {
        let (weather, _) = weather(vec![Ok(forecast(15))]);
        let before = weather.report().revision;
        weather.refresh_if_due(Instant::now());
        assert!(weather.report().revision >= before + 2, "updating, then up to date");
    }

    #[test]
    fn a_percentage_stops_at_a_hundred() {
        assert_eq!(Percent::new(100).map(Percent::value), Some(100));
        assert_eq!(Percent::new(101), None);
        assert_eq!(Percent::saturating(250), Percent::new(100).unwrap());
    }

    #[test]
    fn a_wind_direction_goes_to_the_nearest_point() {
        let points = [0, 22, 23, 90, 180, 214, 292, 337, 338, 359, 360].map(CompassPoint::from_degrees);
        use CompassPoint::*;
        assert_eq!(points, [North, North, NorthEast, East, South, SouthWest, West, NorthWest, North, North, North]);
    }

    #[test]
    fn a_day_is_found_by_its_date() {
        let saturday = Date::new(2026, 9, 26).unwrap();
        let week = (0..7).map(|n| DayForecast {
                date: saturday.plus_days(n),
                sky: Sky::Clear,
                low: Degrees(n as i16),
                high: Degrees(20),
                rain_chance: None,
                sunrise: None,
                sunset: None,
            }).collect();
        let forecast = Forecast { week, ..forecast(15) };
        assert_eq!(forecast.day(saturday.plus_days(2)).map(|day| day.low), Some(Degrees(2)));
        assert_eq!(forecast.day(saturday.plus_days(-1)), None);
        assert_eq!(forecast.day(saturday.plus_days(7)), None);
    }

    #[test]
    fn the_hours_shown_start_with_the_one_under_way() {
        let saturday = Date::new(2026, 9, 26).unwrap();
        let at = |date: Date, hour: u8, minute: u8| LocalTime { date, time_of_day: TimeOfDay::new(hour, minute).unwrap(), second: 0 };
        let hours = [(saturday, 22), (saturday, 23), (saturday.plus_days(1), 0), (saturday.plus_days(1), 1)]
            .iter()
            .map(|&(date, hour)| HourForecast { start: at(date, hour, 0), sky: Sky::Clear, temperature: Degrees(hour.into()), rain_chance: None, precipitation: None })
            .collect();
        let forecast = Forecast { hours, ..forecast(15) };
        let from = |now: LocalTime| forecast.hours_from(now).map(|hour| hour.temperature.0).collect::<Vec<_>>();
        assert_eq!(from(at(saturday, 23, 59)), [23, 0, 1]);
        assert_eq!(from(at(saturday.plus_days(1), 0, 0)), [0, 1]);
        assert_eq!(from(at(saturday, 21, 10)), [22, 23, 0, 1]);
        assert!(from(at(saturday.plus_days(1), 2, 0)).is_empty());
    }
}
