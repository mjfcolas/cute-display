//! The aircraft around a place, watched only while the radar app is in front: nobody is
//! served by fetching traffic every fifteen seconds into a screen nobody looks at.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::apps::{App, Foreground};
use crate::fetch::{FetchStatus, Unavailable};
use crate::place::{GeoPoint, PlaceSource};

pub const REFRESH_EVERY: Duration = Duration::from_secs(15);
pub const RETRY_AFTER: Duration = Duration::from_secs(30);
/// The nearest ones are kept; the heap has room for a busy sky, not an unbounded one.
pub const MAX_AIRCRAFT: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Altitude {
    Ground,
    Feet(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aircraft {
    pub callsign: Option<String>,
    /// As painted on the airframe, `F-GKXA`.
    pub registration: Option<String>,
    pub point: GeoPoint,
    pub altitude: Option<Altitude>,
    /// Clockwise from north.
    pub track_degrees: Option<f32>,
}

/// How far the radar looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Range {
    FiveKm,
    TenKm,
    #[default]
    TwentyFiveKm,
    FiftyKm,
    HundredKm,
}

impl Range {
    const ALL: [Range; 5] = [Range::FiveKm, Range::TenKm, Range::TwentyFiveKm, Range::FiftyKm, Range::HundredKm];

    pub fn km(self) -> u32 {
        match self {
            Self::FiveKm => 5,
            Self::TenKm => 10,
            Self::TwentyFiveKm => 25,
            Self::FiftyKm => 50,
            Self::HundredKm => 100,
        }
    }

    /// The next wider range, or this one at the widest.
    pub fn wider(self) -> Self {
        Self::ALL.into_iter().find(|&r| r > self).unwrap_or(self)
    }

    /// The next closer range, or this one at the closest.
    pub fn closer(self) -> Self {
        Self::ALL.into_iter().rev().find(|&r| r < self).unwrap_or(self)
    }
}

pub trait AirTrafficSource: Send {
    fn nearby(&mut self, center: GeoPoint, radius_km: u32) -> Result<Vec<Aircraft>, Unavailable>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Airport {
    /// ICAO where it has one, `LFPG`.
    pub code: String,
    pub point: GeoPoint,
    /// Whoever set the radar up wants this one named on the scope.
    pub labelled: bool,
}

pub trait AirportSource: Send {
    /// Empty when none are known.
    fn airports(&mut self) -> Vec<Airport>;
}

#[derive(Clone, Debug)]
pub struct RadarReport {
    pub place: Option<String>,
    pub center: Option<GeoPoint>,
    pub range: Range,
    /// Nearest first.
    pub aircraft: Vec<Aircraft>,
    pub airports: Vec<Airport>,
    pub fetched_at: Option<Instant>,
    pub status: FetchStatus,
    /// Moves on at every change, so a screen knows when to redraw.
    pub revision: u64,
}

struct Sources {
    place: Box<dyn PlaceSource>,
    traffic: Box<dyn AirTrafficSource>,
    airports: Box<dyn AirportSource>,
}

struct State {
    report: RadarReport,
    last_attempt: Option<Instant>,
    requested: bool,
}

/// Every clone is the same radar.
#[derive(Clone)]
pub struct Radar {
    state: Arc<Mutex<State>>,
    sources: Arc<Mutex<Sources>>,
    foreground: Foreground,
}

impl Radar {
    pub fn new(
        place: Box<dyn PlaceSource>,
        traffic: Box<dyn AirTrafficSource>,
        airports: Box<dyn AirportSource>,
        foreground: Foreground,
    ) -> Self {
        let report = RadarReport {
            place: None,
            center: None,
            range: Range::default(),
            aircraft: Vec::new(),
            airports: Vec::new(),
            fetched_at: None,
            status: FetchStatus::NeverFetched,
            revision: 0,
        };
        Self {
            state: Arc::new(Mutex::new(State { report, last_attempt: None, requested: false })),
            sources: Arc::new(Mutex::new(Sources { place, traffic, airports })),
            foreground,
        }
    }

    pub fn report(&self) -> RadarReport {
        self.lock_state().report.clone()
    }

    /// A new range is fetched at once, since what was fetched covers another radius.
    pub fn set_range(&self, range: Range) {
        self.change(|state| {
            if state.report.range != range {
                state.report.range = range;
                state.requested = true;
            }
        });
    }

    pub fn request_refresh(&self) {
        self.lock_state().requested = true;
    }

    pub fn is_due(&self, now: Instant) -> bool {
        if self.foreground.app() != App::Radar {
            return false;
        }
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

    /// Fetches if the radar is in front and it is time to. Blocks for as long as the
    /// fetch takes, but never holds up whoever reads the report meanwhile.
    pub fn refresh_if_due(&self, now: Instant) {
        if !self.is_due(now) {
            return;
        }
        let mut range = Range::default();
        self.change(|state| {
            state.report.status = FetchStatus::Updating;
            state.requested = false;
            state.last_attempt = Some(now);
            range = state.report.range;
        });

        let mut sources = self.sources.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(place) = sources.place.place() else {
            drop(sources);
            self.change(|state| {
                state.report.place = None;
                state.report.center = None;
                state.report.status = FetchStatus::NoPlace;
            });
            return;
        };
        let fetched = sources.traffic.nearby(place.point, range.km());
        let airports = sources.airports.airports();
        drop(sources);

        self.change(|state| {
            state.report.place = Some(place.name);
            state.report.center = Some(place.point);
            state.report.airports = airports;
            match fetched {
                Ok(mut aircraft) => {
                    aircraft.sort_by(|a, b| a.point.distance_km(place.point).total_cmp(&b.point.distance_km(place.point)));
                    aircraft.truncate(MAX_AIRCRAFT);
                    state.report.aircraft = aircraft;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::Place;

    const NOTRE_DAME: GeoPoint = GeoPoint { latitude: 48.8530, longitude: 2.3499 };

    struct Orly;

    impl AirportSource for Orly {
        fn airports(&mut self) -> Vec<Airport> {
            vec![Airport { code: "LFPO".into(), point: GeoPoint { latitude: 48.7233, longitude: 2.3794 }, labelled: true }]
        }
    }

    struct NotreDame;

    impl PlaceSource for NotreDame {
        fn place(&mut self) -> Option<Place> {
            Some(Place { name: "Notre-Dame".into(), point: NOTRE_DAME })
        }
    }

    /// Answers with the same traffic every time, and remembers each radius asked for.
    #[derive(Clone)]
    struct Sky(Arc<Mutex<Vec<u32>>>, Vec<Aircraft>);

    impl AirTrafficSource for Sky {
        fn nearby(&mut self, _: GeoPoint, radius_km: u32) -> Result<Vec<Aircraft>, Unavailable> {
            self.0.lock().unwrap().push(radius_km);
            Ok(self.1.clone())
        }
    }

    fn aircraft(name: &str, north_degrees: f64) -> Aircraft {
        let point = GeoPoint { latitude: NOTRE_DAME.latitude + north_degrees, longitude: NOTRE_DAME.longitude };
        Aircraft { callsign: Some(name.into()), registration: None, point, altitude: Some(Altitude::Feet(35_000)), track_degrees: Some(90.0) }
    }

    fn radar(traffic: Vec<Aircraft>, front: App) -> (Radar, Foreground, Arc<Mutex<Vec<u32>>>) {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let foreground = Foreground::new(front);
        let radar = Radar::new(Box::new(NotreDame), Box::new(Sky(Arc::clone(&asked), traffic)), Box::new(Orly), foreground.clone());
        (radar, foreground, asked)
    }

    #[test]
    fn nothing_is_fetched_while_the_radar_is_not_in_front() {
        let (radar, foreground, asked) = radar(vec![], App::Weather);
        radar.refresh_if_due(Instant::now());
        assert!(asked.lock().unwrap().is_empty());
        foreground.bring_to_front(App::Radar);
        radar.refresh_if_due(Instant::now());
        assert_eq!(asked.lock().unwrap().len(), 1);
    }

    #[test]
    fn it_fetches_every_fifteen_seconds_while_in_front() {
        let (radar, _, asked) = radar(vec![], App::Radar);
        let start = Instant::now();
        radar.refresh_if_due(start);
        radar.refresh_if_due(start + REFRESH_EVERY - Duration::from_secs(1));
        assert_eq!(asked.lock().unwrap().len(), 1);
        radar.refresh_if_due(start + REFRESH_EVERY);
        assert_eq!(asked.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_new_range_is_fetched_at_once_for_its_radius() {
        let (radar, _, asked) = radar(vec![], App::Radar);
        let start = Instant::now();
        radar.refresh_if_due(start);
        radar.set_range(Range::default().wider());
        radar.refresh_if_due(start + Duration::from_secs(1));
        assert_eq!(*asked.lock().unwrap(), [25, 50]);
    }

    #[test]
    fn aircraft_are_kept_nearest_first_and_no_more_than_the_cap() {
        let mut traffic: Vec<Aircraft> = (0..MAX_AIRCRAFT + 10).rev().map(|n| aircraft(&format!("F{n}"), (n + 1) as f64 * 0.001)).collect();
        traffic.push(aircraft("CLOSEST", 0.0));
        let (radar, _, _) = radar(traffic, App::Radar);
        radar.refresh_if_due(Instant::now());
        let report = radar.report();
        assert_eq!(report.aircraft.len(), MAX_AIRCRAFT);
        assert_eq!(report.aircraft[0].callsign.as_deref(), Some("CLOSEST"));
        assert_eq!(report.status, FetchStatus::UpToDate);
        assert_eq!(report.airports.iter().map(|a| a.code.as_str()).collect::<Vec<_>>(), ["LFPO"]);
    }

    #[test]
    fn the_range_steps_and_stops_at_both_ends() {
        assert_eq!(Range::FiveKm.closer(), Range::FiveKm);
        assert_eq!(Range::HundredKm.wider(), Range::HundredKm);
        assert_eq!(Range::TwentyFiveKm.wider().km(), 50);
        assert_eq!(Range::TwentyFiveKm.closer().km(), 10);
    }
}
