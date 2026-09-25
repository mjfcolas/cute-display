//! Writes one screen of the app as a raw frame:
//!
//!   cargo run -p ui --example app_screen -- out.fb [system|weather|weather-week|counter|echo|ping]

use std::time::Instant;

use domain::apps::{App, Foreground};
use domain::calendar::Date;
use domain::counter::Counter;
use domain::ping::Ping;
use domain::settings::{Settings, SettingsRecord, SettingsStore};
use domain::weather::{DayForecast, Degrees, Forecast, ForecastSource, Location, LocationSource, Sky, Today, Unavailable, Weather};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH};
use ui::apps::{CounterScreen, EchoScreen, PingScreen, SystemScreen, WeatherScreen};
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

impl LocationSource for Montreal {
    fn location(&mut self) -> Option<Location> {
        Some(Location { place: "Montréal".into(), latitude: 45.5, longitude: -73.57 })
    }
}

struct Sample;

impl ForecastSource for Sample {
    fn fetch(&mut self, _: &Location) -> Result<Forecast, Unavailable> {
        let days = [(Sky::PartlyCloudy, 11, 21), (Sky::Rain, 12, 17), (Sky::Cloudy, 10, 19), (Sky::Clear, 10, 23), (Sky::Fog, 8, 16), (Sky::Snow, -3, 3), (Sky::Storm, 12, 18)];
        let week = days
            .iter()
            .enumerate()
            .map(|(n, &(sky, low, high))| DayForecast { date: Date { year: 2026, month: 9, day: 25 + n as u8 }, sky, low: Degrees(low), high: Degrees(high) })
            .collect();
        Ok(Forecast { today: Today { sky: Sky::PartlyCloudy, now: Degrees(19), low: Degrees(11), high: Degrees(21) }, week })
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).map_or("/tmp/cute-display.fb", String::as_str);
    let screen = args.get(2).map_or("system", String::as_str);

    let foreground = Foreground::new(App::Counter);
    let counter = Counter::new();
    counter.add(12);
    let ping = Ping::new();
    ping.trigger();
    let settings = Settings::load(Box::new(Nowhere));
    let weather = Weather::new(Box::new(Montreal), Box::new(Sample));
    weather.refresh_if_due(Instant::now());
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
        Box::new(SystemScreen::new(foreground.clone(), settings)),
        Box::new(WeatherScreen::new(weather)),
        Box::new(CounterScreen::new(counter)),
        Box::new(EchoScreen::default()),
        Box::new(PingScreen::new(ping)),
    ];
    let Some(mut shell) = Shell::new(foreground.clone(), screens) else {
        return Ok(());
    };

    match screen {
        "counter" => {}
        "weather" => foreground.bring_to_front(App::Weather),
        "weather-week" => {
            foreground.bring_to_front(App::Weather);
            shell.on_sample(&ControlsSample { detents: 1, ..Default::default() }, core::time::Duration::ZERO);
        }
        "echo" => foreground.bring_to_front(App::Echo),
        "ping" => foreground.bring_to_front(App::Ping),
        _ => foreground.open_system(),
    }

    let mut frame = Frame::blank();
    shell.draw(&mut frame, Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into())));
    std::fs::write(out, frame.as_bytes())
}
