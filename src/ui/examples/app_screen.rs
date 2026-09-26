//! Writes one screen of the app as a raw frame:
//!
//!   cargo run -p ui --example app_screen -- out.fb [system|weather|weather-week|radar]

use std::time::Instant;

use domain::apps::{App, Foreground};
use domain::calendar::Date;
use domain::radar::{AirTrafficSource, Aircraft, Airport, AirportSource, Altitude, Radar};
use domain::settings::{Settings, SettingsRecord, SettingsStore};
use domain::fetch::Unavailable;
use domain::place::{GeoPoint, Place, PlaceSource};
use domain::weather::{DayForecast, Degrees, Forecast, ForecastSource, Sky, Today, Weather};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH};
use ui::apps::{RadarScreen, SystemScreen, WeatherScreen};
use ui::controls::ControlsSample;
use ui::{AppScreen, Shell};

struct Nowhere;

impl SettingsStore for Nowhere {
    fn load(&mut self) -> Option<SettingsRecord> {
        None
    }
    fn save(&mut self, _: &SettingsRecord) {}
}

struct Montreal;

impl PlaceSource for Montreal {
    fn place(&mut self) -> Option<Place> {
        Some(Place { name: "Montréal".into(), point: GeoPoint { latitude: 45.5, longitude: -73.57 } })
    }
}

struct Sample;

impl ForecastSource for Sample {
    fn fetch(&mut self, _: &Place) -> Result<Forecast, Unavailable> {
        let days = [(Sky::PartlyCloudy, 11, 21), (Sky::Rain, 12, 17), (Sky::Cloudy, 10, 19), (Sky::Clear, 10, 23), (Sky::Fog, 8, 16), (Sky::Snow, -3, 3), (Sky::Storm, 12, 18)];
        let week = days
            .iter()
            .enumerate()
            .filter_map(|(n, &(sky, low, high))| Some(DayForecast { date: Date::new(2026, 9, 25 + n as u8)?, sky, low: Degrees(low), high: Degrees(high) }))
            .collect();
        Ok(Forecast { today: Today { sky: Sky::PartlyCloudy, now: Degrees(19), low: Degrees(11), high: Degrees(21) }, week })
    }
}

const NOTRE_DAME: GeoPoint = GeoPoint { latitude: 48.8530, longitude: 2.3499 };

struct NotreDame;

impl PlaceSource for NotreDame {
    fn place(&mut self) -> Option<Place> {
        Some(Place { name: "Notre-Dame".into(), point: NOTRE_DAME })
    }
}

struct Traffic;

impl AirTrafficSource for Traffic {
    fn nearby(&mut self, _: GeoPoint, _: u32) -> Result<Vec<Aircraft>, Unavailable> {
        let flights = [
            ("AFR1234", "F-HEPA", 4.0, 3.0, 12_000, 250.0),
            ("EZY42QK", "G-EZOA", -9.0, 6.0, 7_500, 80.0),
            ("RYR8TW", "EI-DWF", 12.0, -14.0, 36_000, 190.0),
            ("BAW316", "G-EUYA", -18.0, -5.0, 24_000, 310.0),
            ("TVF81PL", "F-HTVA", 2.0, -8.0, 3_200, 20.0),
            ("DLH9AB", "D-AIZA", 20.0, 16.0, 38_000, 135.0),
        ];
        Ok(flights
            .iter()
            .map(|&(callsign, registration, east, north, feet, track)| Aircraft {
                callsign: Some(callsign.into()),
                registration: Some(registration.into()),
                point: GeoPoint {
                    latitude: NOTRE_DAME.latitude + north / 111.2,
                    longitude: NOTRE_DAME.longitude + east / (111.2 * NOTRE_DAME.latitude.to_radians().cos()),
                },
                altitude: Some(Altitude::Feet(feet)),
                track_degrees: Some(track),
            })
            .collect())
    }
}

struct AroundParis;

impl AirportSource for AroundParis {
    fn airports(&mut self) -> Vec<Airport> {
        [("LFPG", 49.0097, 2.5479), ("LFPO", 48.7233, 2.3794), ("LFPB", 48.9694, 2.4414)]
            .iter()
            .map(|&(code, latitude, longitude)| Airport {
                code: code.into(),
                point: GeoPoint { latitude, longitude },
                labelled: matches!(code, "LFPG" | "LFPO" | "LFPB"),
            })
            .collect()
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).map_or("/tmp/cute-display.fb", String::as_str);
    let screen = args.get(2).map_or("system", String::as_str);

    let foreground = Foreground::new(App::Weather);
    let settings = Settings::load(Box::new(Nowhere));
    let weather = Weather::new(Box::new(Montreal), Box::new(Sample));
    weather.refresh_if_due(Instant::now());
    let radar = Radar::new(Box::new(NotreDame), Box::new(Traffic), Box::new(AroundParis), foreground.clone());
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
        Box::new(SystemScreen::new(foreground.clone(), settings)),
        Box::new(WeatherScreen::new(weather)),
        Box::new(RadarScreen::new(radar.clone())),
    ];
    let Some(mut shell) = Shell::new(foreground.clone(), screens) else {
        return Ok(());
    };

    match screen {
        "weather" => foreground.bring_to_front(App::Weather),
        "radar" => {
            foreground.bring_to_front(App::Radar);
            radar.refresh_if_due(Instant::now());
        }
        "weather-week" => {
            foreground.bring_to_front(App::Weather);
            shell.on_sample(&ControlsSample { detents: 1, ..Default::default() }, core::time::Duration::ZERO);
        }
        _ => foreground.open_system(),
    }

    let mut frame = Frame::blank();
    shell.draw(&mut frame, Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into())));
    std::fs::write(out, frame.as_bytes())
}
