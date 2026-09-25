//! The weather at one place: today, and the days ahead. Fetched from outside every hour,
//! and on request; a failed fetch keeps the last forecast rather than showing nothing.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::calendar::Date;

pub const REFRESH_EVERY: Duration = Duration::from_secs(60 * 60);
pub const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// What the place is called on the glass.
    pub place: String,
    pub latitude: f64,
    pub longitude: f64,
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct Today {
    pub sky: Sky,
    pub now: Degrees,
    pub low: Degrees,
    pub high: Degrees,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayForecast {
    pub date: Date,
    pub sky: Sky,
    pub low: Degrees,
    pub high: Degrees,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub today: Today,
    /// Today first.
    pub week: Vec<DayForecast>,
}

/// Why there is no forecast, in words a person can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unavailable(pub String);

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub trait LocationSource: Send {
    /// `None` when no place has been set.
    fn location(&mut self) -> Option<Location>;
}

pub trait ForecastSource: Send {
    fn fetch(&mut self, location: &Location) -> Result<Forecast, Unavailable>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    NeverFetched,
    Updating,
    UpToDate,
    NoLocation,
    Failed(Unavailable),
}

/// Everything the weather is at one moment.
#[derive(Clone, Debug)]
pub struct WeatherReport {
    pub place: Option<String>,
    pub forecast: Option<Forecast>,
    /// When `forecast` was fetched.
    pub fetched_at: Option<Instant>,
    pub status: Status,
    /// Moves on at every change, so a screen knows when to redraw.
    pub revision: u64,
}

struct Sources {
    location: Box<dyn LocationSource>,
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
    pub fn new(location: Box<dyn LocationSource>, forecast: Box<dyn ForecastSource>) -> Self {
        let report = WeatherReport { place: None, forecast: None, fetched_at: None, status: Status::NeverFetched, revision: 0 };
        Self {
            state: Arc::new(Mutex::new(State { report, last_attempt: None, requested: false })),
            sources: Arc::new(Mutex::new(Sources { location, forecast })),
        }
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
            Status::Failed(_) | Status::NoLocation => RETRY_AFTER,
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
            state.report.status = Status::Updating;
            state.requested = false;
            state.last_attempt = Some(now);
        });

        let mut sources = self.sources.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(location) = sources.location.location() else {
            drop(sources);
            self.change(|state| {
                state.report.place = None;
                state.report.status = Status::NoLocation;
            });
            return;
        };
        let fetched = sources.forecast.fetch(&location);
        drop(sources);

        self.change(|state| {
            state.report.place = Some(location.place);
            match fetched {
                Ok(forecast) => {
                    state.report.forecast = Some(forecast);
                    state.report.fetched_at = Some(now);
                    state.report.status = Status::UpToDate;
                }
                Err(unavailable) => state.report.status = Status::Failed(unavailable),
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Option<Location>);

    impl LocationSource for Fixed {
        fn location(&mut self) -> Option<Location> {
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
        fn fetch(&mut self, _: &Location) -> Result<Forecast, Unavailable> {
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

    fn paris() -> Option<Location> {
        Some(Location { place: "Paris".into(), latitude: 48.85, longitude: 2.35 })
    }

    fn forecast(now: i16) -> Forecast {
        let today = Today { sky: Sky::Clear, now: Degrees(now), low: Degrees(10), high: Degrees(20) };
        Forecast { today, week: vec![] }
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
        assert_eq!(report.status, Status::UpToDate);
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
        assert_eq!(report.status, Status::Failed(Unavailable("no Wi-Fi".into())));
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
        assert_eq!(weather.report().status, Status::NoLocation);
        assert_eq!(source.fetches(), 0);
    }

    #[test]
    fn every_change_moves_the_revision_on() {
        let (weather, _) = weather(vec![Ok(forecast(15))]);
        let before = weather.report().revision;
        weather.refresh_if_due(Instant::now());
        assert!(weather.report().revision >= before + 2, "updating, then up to date");
    }
}
