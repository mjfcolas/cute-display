//! Writes one screen of the app as a raw frame:
//!
//!   cargo run -p ui --example app_screen -- out.fb [system|alarm|alarm-days|weather|weather-week|radar]

use std::time::Instant;

use domain::alarm::{AlarmClock, AlarmSchedule, AlarmScheduleStore, Ringer, Volume};
use domain::apps::{App, Foreground};
use domain::calendar::{Date, Weekday};
use domain::clock::{Clock, TimeKeeper, TimeSource, TimeZoneSource};
use domain::time::{LocalTime, TimeOfDay, UtcTime};
use domain::time_zone::TimeZone;
use domain::radar::{AirTrafficSource, Aircraft, Airport, AirportSource, Altitude, Radar};
use domain::settings::{Settings, SettingsRecord, SettingsStore};
use domain::fetch::Unavailable;
use domain::place::{GeoPoint, Place, PlaceSource};
use domain::weather::{
    CompassPoint, DayForecast, Degrees, Forecast, ForecastSource, Hectopascals, HourForecast, KilometresPerHour, Millimetres, Percent, Sky, Today,
    Weather, Wind,
};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH};
use ui::apps::{AlarmScreen, RadarScreen, SystemScreen, WeatherScreen};
use ui::controls::{ButtonSample, ControlsSample};
use ui::{AppScreen, Shell};

struct Nowhere;

impl SettingsStore for Nowhere {
    fn load(&mut self) -> Option<SettingsRecord> {
        None
    }
    fn save(&mut self, _: &SettingsRecord) {}
}

/// Friday 25 September 2026, 21:47 in Paris.
struct FridayEvening;

impl TimeKeeper for FridayEvening {
    fn read(&mut self) -> Option<UtcTime> {
        Some(UtcTime::from_unix_seconds(1_790_365_620))
    }
    fn set(&mut self, _: UtcTime) {}
}

impl TimeSource for FridayEvening {
    fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
        Err(Unavailable("offline".into()))
    }
}

impl TimeZoneSource for FridayEvening {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        Ok(None)
    }
}

impl AlarmScheduleStore for Nowhere {
    fn load(&mut self) -> Option<AlarmSchedule> {
        None
    }
    fn save(&mut self, _: &AlarmSchedule) {}
}

impl Ringer for Nowhere {
    fn ring(&mut self, _: Volume) {}
    fn silence(&mut self) {}
}

/// Enabled, at 7:00 on weekdays and 9:30 on Saturdays.
fn alarm_clock(foreground: &Foreground) -> AlarmClock {
    let alarm = AlarmClock::new(Box::new(Nowhere), Box::new(Nowhere), foreground.clone());
    for day in [Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday] {
        alarm.set_time_on(day, TimeOfDay::new(7, 0));
    }
    alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(9, 30));
    alarm.switch_on();
    alarm
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
        let days = [
            (Sky::PartlyCloudy, 11, 21, 20),
            (Sky::Rain, 12, 17, 85),
            (Sky::Cloudy, 10, 19, 30),
            (Sky::Clear, 10, 23, 0),
            (Sky::Fog, 8, 16, 10),
            (Sky::Snow, -3, 3, 60),
            (Sky::Storm, 12, 18, 75),
        ];
        let first = Date::new(2026, 9, 25).ok_or_else(|| Unavailable("no such date".into()))?;
        let week = days
            .iter()
            .zip(0..)
            .map(|(&(sky, low, high, rain), n)| DayForecast {
                date: first.plus_days(n),
                sky,
                low: Degrees(low),
                high: Degrees(high),
                rain_chance: Percent::new(rain),
                sunrise: TimeOfDay::new(7, 40),
                sunset: TimeOfDay::new(19, 43),
            })
            .collect();
        let hours = [
            (Sky::PartlyCloudy, 17, 0, 0),
            (Sky::Clear, 15, 0, 0),
            (Sky::Clear, 14, 5, 0),
            (Sky::Cloudy, 13, 10, 0),
            (Sky::Cloudy, 13, 20, 1),
            (Sky::Rain, 12, 60, 8),
            (Sky::Rain, 12, 75, 26),
            (Sky::Rain, 12, 70, 14),
            (Sky::Cloudy, 12, 35, 3),
            (Sky::Cloudy, 11, 15, 0),
            (Sky::PartlyCloudy, 12, 5, 0),
            (Sky::Clear, 14, 0, 0),
            (Sky::Clear, 16, 0, 0),
        ];
        let friday_at_nine = LocalTime { date: first, time_of_day: TimeOfDay::MIDNIGHT, second: 0 }.seconds_since_epoch() + 21 * 3600;
        let hours = hours
            .iter()
            .zip(0..)
            .map(|(&(sky, temperature, rain, fallen), n)| HourForecast {
                start: LocalTime::from_seconds_since_epoch(friday_at_nine + n * 3600),
                sky,
                temperature: Degrees(temperature),
                rain_chance: Percent::new(rain),
                precipitation: Some(Millimetres::from_tenths(fallen)),
            })
            .collect();
        let today = Today {
            sky: Sky::PartlyCloudy,
            now: Degrees(19),
            feels_like: Degrees(17),
            humidity: Percent::saturating(64),
            pressure: Hectopascals(1016),
            wind: Wind { speed: KilometresPerHour(12), from: CompassPoint::SouthWest },
        };
        Ok(Forecast { today, hours, week })
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
    let clock = Clock::new(Box::new(FridayEvening), Box::new(FridayEvening), Box::new(FridayEvening));
    clock.tick(Instant::now());
    let alarm = alarm_clock(&foreground);
    if let Some(now) = clock.now() {
        alarm.tick(now);
    }
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
        Box::new(SystemScreen::new(foreground.clone(), settings)),
        Box::new(AlarmScreen::new(alarm, clock.clone(), weather.clone())),
        Box::new(WeatherScreen::new(weather, clock)),
        Box::new(RadarScreen::new(radar.clone())),
    ];
    let Some(mut shell) = Shell::new(foreground.clone(), screens) else {
        return Ok(());
    };

    match screen {
        "alarm" => foreground.bring_to_front(App::Alarm),
        "alarm-days" => {
            foreground.bring_to_front(App::Alarm);
            let press = ButtonSample { presses: 1, held: false };
            shell.on_sample(&ControlsSample { long: press, ..Default::default() }, core::time::Duration::ZERO);
            shell.on_sample(&ControlsSample { long: press, ..Default::default() }, core::time::Duration::ZERO);
        }
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
