use domain::place::CompassPoint;
use ui::{Describe, Description};

use super::distance;
use super::ui_state::{FromPlace, RadarUiState};

impl Describe for RadarUiState {
    fn describe(&self) -> Description {
        let mut description = Description::default();
        description.say("title", &self.title);
        description.say("range", &format!("{} km", self.range_km));
        for airport in &self.airports {
            description.say("airport", &format!("{} {}", airport.label.as_deref().unwrap_or("?"), distance_and_direction(airport.from_place)));
        }
        for aircraft in &self.aircraft {
            let track = aircraft.track_degrees.map_or_else(|| "no track".into(), |track| format!("track {track:.0}°"));
            description.say("aircraft", &format!("{} {} {track}", aircraft.label.as_deref().unwrap_or("?"), distance_and_direction(aircraft.from_place)));
        }
        for listed in &self.nearest {
            description.say("nearest", &format!("{} {}", listed.identity, listed.whereabouts));
        }
        if let Some(trouble) = &self.trouble {
            description.say("trouble", trouble);
        }
        description.say("hint", &self.hints.join(" "));
        description.say("credit", self.credit);
        description
    }
}

fn distance_and_direction(from: FromPlace) -> String {
    format!("{} {}", distance(from.east_km.hypot(from.north_km)), CompassPoint::toward(from.east_km, from.north_km).abbreviation())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screen::{AircraftMark, AirportMark, ListedAircraft};

    #[test]
    fn each_mark_is_said_by_how_far_and_which_way_it_is() {
        let state = RadarUiState {
            title: "Notre-Dame".into(),
            range_km: 25,
            airports: vec![AirportMark { from_place: FromPlace { east_km: -6.0, north_km: -15.0 }, label: Some("LFPO".into()) }],
            aircraft: vec![
                AircraftMark { from_place: FromPlace { east_km: 8.0, north_km: 8.0 }, track_degrees: Some(45.0), label: Some("XA".into()) },
                AircraftMark { from_place: FromPlace { east_km: 0.0, north_km: -3.0 }, track_degrees: None, label: None },
            ],
            nearest: vec![ListedAircraft { identity: "XA AFR1234".into(), whereabouts: "   35000 ft  11 km".into() }],
            trouble: Some("offline: no Wi-Fi".into()),
            hints: ["wheel: range", "long: update"],
            credit: "data: adsb.fi",
        };
        assert_eq!(
            state.describe().text(),
            [
                "title Notre-Dame",
                "range 25 km",
                "airport LFPO 16 km S",
                "aircraft XA 11 km NE track 45°",
                "aircraft ? 3.0 km S no track",
                "nearest XA AFR1234 35000 ft 11 km",
                "trouble offline: no Wi-Fi",
                "hint wheel: range long: update",
                "credit data: adsb.fi",
            ]
        );
    }
}
