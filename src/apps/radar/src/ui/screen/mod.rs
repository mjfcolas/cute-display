mod drawing;
mod ui_state;

use domain::fetch::FetchStatus;
use domain::place::GeoPoint;
use ui::controls::{Button, Input};
use ui::Screen;

pub use ui_state::{AircraftMark, AirportMark, FromPlace, ListedAircraft, RadarUiState};

use crate::domain::radar::{Aircraft, Airport, Altitude, Radar, RadarReport};
use crate::ui::radar_view::short_registration;

const NEAREST_LISTED: usize = 5;

pub struct RadarScreen {
    radar: Radar,
}

impl RadarScreen {
    pub fn new(radar: Radar) -> Self {
        Self { radar }
    }
}

impl Screen for RadarScreen {
    type UiState = RadarUiState;

    fn ui_state(&self) -> RadarUiState {
        let report = self.radar.report();
        let range_km = report.range.km();
        let (airports, aircraft, nearest) = match report.center {
            Some(center) => {
                let in_range = |point: GeoPoint| point.distance_km(center) <= f64::from(range_km);
                let within: Vec<&Aircraft> = report.aircraft.iter().filter(|a| in_range(a.point)).collect();
                (
                    report.airports.iter().filter(|a| in_range(a.point)).map(|airport| airport_mark(airport, center)).collect(),
                    within.iter().map(|aircraft| aircraft_mark(aircraft, center)).collect(),
                    within.iter().take(NEAREST_LISTED).map(|aircraft| listed(aircraft, center)).collect(),
                )
            }
            None => (Vec::new(), Vec::new(), Vec::new()),
        };
        RadarUiState {
            title: report.place.clone().unwrap_or_else(|| "Radar".into()),
            range_km,
            airports,
            aircraft,
            nearest,
            trouble: trouble(&report),
        }
    }

    fn version(&self) -> u64 {
        self.radar.report().revision
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => {
                let mut range = self.radar.report().range;
                for _ in 0..detents.unsigned_abs() {
                    range = if detents > 0 { range.wider() } else { range.closer() };
                }
                self.radar.set_range(range);
            }
            Input::Press(Button::Long) => self.radar.request_refresh(),
            Input::Press(Button::Yellow) | Input::HoldYellowAndLong => {}
        }
    }
}

fn from_place(point: GeoPoint, center: GeoPoint) -> FromPlace {
    let (east_km, north_km) = point.offset_from(center);
    FromPlace { east_km, north_km }
}

fn airport_mark(airport: &Airport, center: GeoPoint) -> AirportMark {
    AirportMark { from_place: from_place(airport.point, center), label: airport.labelled.then(|| airport.code.clone()) }
}

fn aircraft_mark(aircraft: &Aircraft, center: GeoPoint) -> AircraftMark {
    AircraftMark {
        from_place: from_place(aircraft.point, center),
        track_degrees: aircraft.track_degrees,
        label: aircraft.registration.as_deref().and_then(short_registration),
    }
}

fn listed(aircraft: &Aircraft, center: GeoPoint) -> ListedAircraft {
    ListedAircraft { identity: identity(aircraft), whereabouts: whereabouts(aircraft, aircraft.point.distance_km(center)) }
}

/// Its two letters first, as on the scope, so the line and the mark can be matched.
fn identity(aircraft: &Aircraft) -> String {
    let letters = aircraft.registration.as_deref().and_then(short_registration).unwrap_or_default();
    format!("{letters:<2} {}", aircraft.callsign.as_deref().unwrap_or("?"))
}

fn whereabouts(aircraft: &Aircraft, distance_km: f64) -> String {
    let altitude = match aircraft.altitude {
        Some(Altitude::Ground) => "ground".to_owned(),
        Some(Altitude::Feet(feet)) => format!("{feet} ft"),
        None => "-".to_owned(),
    };
    let distance = if distance_km < 10.0 { format!("{distance_km:.1} km") } else { format!("{distance_km:.0} km") };
    format!("   {altitude}  {distance}")
}

fn trouble(report: &RadarReport) -> Option<String> {
    match &report.status {
        FetchStatus::NeverFetched | FetchStatus::Updating if report.fetched_at.is_none() => Some("looking...".to_owned()),
        FetchStatus::NoPlace => Some("no place: put cute-display/general.conf".to_owned()),
        FetchStatus::Failed(why) => Some(format!("offline: {why}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use domain::apps::Foreground;
    use domain::fetch::Unavailable;
    use domain::place::Place;
    use domain_testing::place::StubPlace;

    use super::*;
    use crate::domain::radar::{AirTrafficSource, Airport, AirportSource};

    const NOTRE_DAME: GeoPoint = GeoPoint { latitude: 48.8530, longitude: 2.3499 };

    #[derive(Clone)]
    struct StubAirTrafficSource(Arc<Mutex<Vec<Aircraft>>>);

    impl AirTrafficSource for StubAirTrafficSource {
        fn nearby(&mut self, _: GeoPoint, _: u32) -> Result<Vec<Aircraft>, Unavailable> {
            Ok(self.0.lock().unwrap().clone())
        }
    }

    fn at(east_km: f64, north_km: f64) -> GeoPoint {
        GeoPoint {
            latitude: NOTRE_DAME.latitude + north_km / 111.2,
            longitude: NOTRE_DAME.longitude + east_km / (111.2 * NOTRE_DAME.latitude.to_radians().cos()),
        }
    }

    fn plane(east_km: f64, north_km: f64) -> Aircraft {
        Aircraft {
            callsign: Some("AFR1234".into()),
            registration: Some("F-GKXA".into()),
            point: at(east_km, north_km),
            altitude: Some(Altitude::Feet(35_000)),
            track_degrees: Some(45.0),
        }
    }

    fn unnamed(aircraft: Aircraft) -> Aircraft {
        Aircraft { callsign: None, registration: None, ..aircraft }
    }

    struct StubAirportSource(Vec<Airport>);

    impl AirportSource for StubAirportSource {
        fn airports(&mut self) -> Vec<Airport> {
            self.0.clone()
        }
    }

    fn radar_with_airports(traffic: Vec<Aircraft>, airports: Vec<Airport>) -> Radar {
        let traffic = Box::new(StubAirTrafficSource(Arc::new(Mutex::new(traffic))));
        let radar = Radar::new(Box::new(StubPlace::at(Place { name: "Notre-Dame".into(), point: NOTRE_DAME })), traffic, Box::new(StubAirportSource(airports)), Foreground::new(crate::ID));
        radar.refresh_if_due(Instant::now());
        radar
    }

    fn radar_with(traffic: Vec<Aircraft>) -> Radar {
        radar_with_airports(traffic, vec![])
    }

    fn screen_with(traffic: Vec<Aircraft>) -> RadarScreen {
        RadarScreen::new(radar_with(traffic))
    }

    #[test]
    fn an_aircraft_is_marked_where_it_is_from_the_place_with_its_letters() {
        let state = screen_with(vec![plane(8.0, -3.0)]).ui_state();
        let [mark] = &state.aircraft[..] else { panic!("{:?}", state.aircraft) };
        let FromPlace { east_km, north_km } = mark.from_place;
        assert!((east_km - 8.0).abs() < 0.1 && (north_km + 3.0).abs() < 0.1, "{mark:?}");
        assert_eq!((mark.track_degrees, mark.label.as_deref()), (Some(45.0), Some("XA")));
        assert_eq!(screen_with(vec![unnamed(plane(8.0, -3.0))]).ui_state().aircraft[0].label, None);
    }

    fn orly() -> Airport {
        Airport { code: "LFPO".into(), point: at(-6.0, -15.0), labelled: false }
    }

    fn airports_shown(airports: Vec<Airport>) -> Vec<AirportMark> {
        RadarScreen::new(radar_with_airports(vec![], airports)).ui_state().airports
    }

    #[test]
    fn an_airport_is_named_only_when_asked() {
        assert_eq!(airports_shown(vec![orly()])[0].label, None);
        assert_eq!(airports_shown(vec![Airport { labelled: true, ..orly() }])[0].label.as_deref(), Some("LFPO"));
    }

    #[test]
    fn an_airport_beyond_the_range_is_not_on_the_scope() {
        assert_eq!(airports_shown(vec![Airport { point: at(30.0, 0.0), ..orly() }]), []);
    }

    #[test]
    fn an_aircraft_beyond_the_range_is_neither_on_the_scope_nor_listed() {
        let state = screen_with(vec![plane(40.0, 0.0)]).ui_state();
        assert_eq!(state.range_km, 25);
        assert_eq!((state.aircraft.len(), state.nearest.len()), (0, 0));
    }

    #[test]
    fn the_wheel_changes_the_range_and_stops_at_its_ends() {
        let mut screen = screen_with(vec![]);
        let mut turn = |detents| {
            Screen::on_input(&mut screen, Input::Turn(detents));
            screen.ui_state().range_km
        };
        assert_eq!(turn(1), 50);
        assert_eq!(turn(10), 100);
        assert_eq!(turn(-10), 5);
    }

    #[test]
    fn a_press_asks_for_an_update() {
        let radar = radar_with(vec![]);
        let now = Instant::now();
        let mut screen = RadarScreen::new(radar.clone());
        assert!(!radar.is_due(now));
        Screen::on_input(&mut screen, Input::Press(Button::Long));
        assert!(radar.is_due(now));
    }

    #[test]
    fn the_nearest_within_the_range_are_listed_by_their_letters_then_where_they_are() {
        let state = screen_with(vec![plane(3.0, 0.0), plane(60.0, 0.0)]).ui_state();
        let listed: Vec<(&str, &str)> = state.nearest.iter().map(|a| (a.identity.as_str(), a.whereabouts.as_str())).collect();
        assert_eq!(listed, [("XA AFR1234", "   35000 ft  3.0 km")]);
        assert_eq!(screen_with(vec![unnamed(plane(3.0, 0.0))]).ui_state().nearest[0].identity, "   ?");
        let crowd: Vec<Aircraft> = (1..9).map(|n| plane(f64::from(n), 0.0)).collect();
        assert_eq!(screen_with(crowd).ui_state().nearest.len(), NEAREST_LISTED);
    }

    #[test]
    fn where_an_aircraft_is_says_its_altitude_and_its_distance() {
        let whereabouts = |aircraft| screen_with(vec![aircraft]).ui_state().nearest[0].whereabouts.clone();
        assert_eq!(whereabouts(Aircraft { altitude: Some(Altitude::Ground), ..plane(3.0, 0.0) }), "   ground  3.0 km");
        assert_eq!(whereabouts(Aircraft { altitude: None, ..plane(3.0, 0.0) }), "   -  3.0 km");
        assert_eq!(whereabouts(plane(12.4, 0.0)), "   35000 ft  12 km", "whole kilometres from 10 on");
    }

    struct StubFailingAirTrafficSource;

    impl AirTrafficSource for StubFailingAirTrafficSource {
        fn nearby(&mut self, _: GeoPoint, _: u32) -> Result<Vec<Aircraft>, Unavailable> {
            Err(Unavailable("no Wi-Fi".into()))
        }
    }

    #[test]
    fn trouble_is_said_until_the_first_answer_and_when_offline() {
        let notre_dame = || Box::new(StubPlace::at(Place { name: "Notre-Dame".into(), point: NOTRE_DAME }));
        let radar = Radar::new(notre_dame(), Box::new(StubFailingAirTrafficSource), Box::new(StubAirportSource(vec![])), Foreground::new(crate::ID));
        assert_eq!(RadarScreen::new(radar.clone()).ui_state().trouble.as_deref(), Some("looking..."));
        radar.refresh_if_due(Instant::now());
        assert_eq!(RadarScreen::new(radar).ui_state().trouble.as_deref(), Some("offline: no Wi-Fi"));
        assert_eq!(screen_with(vec![]).ui_state().trouble, None, "an answer, no trouble");
    }

    #[test]
    fn trouble_is_said_and_without_a_place_nothing_is_marked() {
        let no_place = Radar::new(
            Box::new(StubPlace::nowhere()),
            Box::new(StubAirTrafficSource(Arc::new(Mutex::new(vec![plane(3.0, 0.0)])))),
            Box::new(StubAirportSource(vec![])),
            Foreground::new(crate::ID),
        );
        no_place.refresh_if_due(Instant::now());
        let state = RadarScreen::new(no_place).ui_state();
        assert_eq!(state.trouble.as_deref(), Some("no place: put cute-display/general.conf"));
        assert_eq!((state.title.as_str(), state.aircraft.len(), state.nearest.len()), ("Radar", 0, 0));
    }
}
