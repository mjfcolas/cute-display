//! Writes one screen of the app as a raw frame:
//!
//!   cargo run -p ui --example app_screen -- out.fb [system|counter|echo|ping]

use domain::apps::{App, Foreground};
use domain::counter::Counter;
use domain::ping::Ping;
use domain::settings::{Settings, SettingsRecord, SettingsStore};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH};
use ui::apps::{CounterScreen, EchoScreen, PingScreen, SystemScreen};
use ui::{AppScreen, Shell};

struct Nowhere;

impl SettingsStore for Nowhere {
    fn load(&mut self) -> Option<SettingsRecord> {
        None
    }
    fn save(&mut self, _: &SettingsRecord) {}
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
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
        Box::new(SystemScreen::new(foreground.clone(), settings)),
        Box::new(CounterScreen::new(counter)),
        Box::new(EchoScreen::default()),
        Box::new(PingScreen::new(ping)),
    ];
    let Some(mut shell) = Shell::new(foreground.clone(), screens) else {
        return Ok(());
    };

    match screen {
        "counter" => {}
        "echo" => foreground.bring_to_front(App::Echo),
        "ping" => foreground.bring_to_front(App::Ping),
        _ => foreground.open_system(),
    }

    let mut frame = Frame::blank();
    shell.draw(&mut frame, Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into())));
    std::fs::write(out, frame.as_bytes())
}
