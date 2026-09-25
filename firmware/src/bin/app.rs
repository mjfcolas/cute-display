use std::io::{self, BufRead};
use std::thread;
use std::time::{Duration, Instant};

use domain::apps::{App, Foreground};
use domain::counter::Counter;
use domain::lighting::Lighting;
use domain::ping::Ping;
use domain::settings::{Settings, SettingsStore};
use domain::weather::Weather;
use drivers::button::Button;
use drivers::esp_system::EspSystem;
use drivers::pcnt_encoder::PcntEncoder;
use drivers::sdmmc_card::SdmmcCard;
use drivers::usb_console;
use drivers::uc8253::Uc8253;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::FreeRtos;
use hal::display::{EpaperDisplay, Frame, Redraw, HEIGHT, VISIBLE_WIDTH};
use hal::input::{PushButton, RotaryEncoder};
use hal::system::SystemMonitor;
use hal::Fault;
use infrastructure::hal_light::HalLight;
use infrastructure::internet::{NoInternet, OnDemandInternet};
use infrastructure::location_file::{LocationFile, NoPlace};
use infrastructure::open_meteo::OpenMeteo;
use infrastructure::settings_file::{SettingsFile, Unkept};
use maintenance::MaintenanceConsole;
use ui::apps::{CounterScreen, EchoScreen, PingScreen, SystemScreen, WeatherScreen};
use ui::controls::{ButtonSample, ControlsSample};
use ui::{AppScreen, ScreenChange, Shell};

use firmware::board::Board;

const BUILD: &str = concat!("build ", env!("BUILD_TIME"), " @", env!("BUILD_GIT"));
const CONTROLS_PERIOD: Duration = Duration::from_millis(20);
const LIGHTING_PERIOD: Duration = Duration::from_millis(100);
const WEATHER_PERIOD: Duration = Duration::from_secs(1);

/// Everything the domain is made of, shared by the threads that use it.
struct Domain {
    foreground: Foreground,
    settings: Settings,
    lighting: Lighting,
    weather: Weather,
    counter: Counter,
    ping: Ping,
}

struct Controls {
    wheel: PcntEncoder,
    wheel_button: Button,
    yellow_button: Button,
    long_button: Button,
}

impl Controls {
    fn sample(&mut self) -> ControlsSample {
        let button = |b: &mut Button| ButtonSample { presses: b.take_presses(), held: b.is_held() };
        ControlsSample {
            detents: self.wheel.take_detents(),
            wheel: button(&mut self.wheel_button),
            yellow: button(&mut self.yellow_button),
            long: button(&mut self.long_button),
        }
    }
}

fn main() -> Result<(), Fault> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // The monitor re-attaches after resetting the chip; anything logged before is lost.
    FreeRtos::delay_ms(1500);
    log::info!("cute-display, {BUILD}");

    let board = Board::bring_up()?;
    let settings_store: Box<dyn SettingsStore> = match &board.sd_card {
        Ok(card) => {
            start_maintenance(card.clone())?;
            Box::new(SettingsFile::new(card.clone()))
        }
        Err(fault) => {
            log::warn!("settings: no SD card ({fault}); they will not survive a power cut");
            Box::new(Unkept)
        }
    };
    let settings = Settings::load(settings_store);
    let lighting = Lighting::new(
        Box::new(HalLight::new(board.front_light)),
        Box::new(HalLight::new(board.reading_lamp)),
        settings.clone(),
    );
    lighting.touched(Instant::now());

    let weather = match &board.sd_card {
        Ok(card) => Weather::new(
            Box::new(LocationFile::new(card.clone())),
            Box::new(OpenMeteo::new(OnDemandInternet::new(board.wifi, board.https, card.clone()))),
        ),
        Err(_) => Weather::new(Box::new(NoPlace), Box::new(OpenMeteo::new(NoInternet("no SD card")))),
    };
    start_weather(weather.clone())?;

    let domain = Domain {
        foreground: Foreground::new(App::Weather),
        settings,
        lighting: lighting.clone(),
        weather,
        counter: Counter::new(),
        ping: Ping::new(),
    };
    let controls = Controls {
        wheel: board.wheel,
        wheel_button: board.wheel_button,
        yellow_button: board.yellow_button,
        long_button: board.long_button,
    };
    let display = board.panel;
    thread::Builder::new()
        .name("ui".into())
        .stack_size(32 * 1024)
        .spawn(move || run_ui(controls, display, domain))
        .map_err(Fault::new)?;

    loop {
        lighting.refresh(Instant::now());
        thread::sleep(LIGHTING_PERIOD);
    }
}

/// Fetches the forecast whenever it is due. On a thread of its own, since a fetch joins
/// Wi-Fi and waits on a server for seconds; the stack is for TLS.
fn start_weather(weather: Weather) -> Result<(), Fault> {
    thread::Builder::new()
        .name("weather".into())
        .stack_size(24 * 1024)
        .spawn(move || loop {
            let now = Instant::now();
            if weather.is_due(now) {
                let system = EspSystem;
                log::info!(
                    "weather: updating, {} KiB free, largest block {} KiB",
                    system.free_heap_bytes() / 1024,
                    system.largest_free_block() / 1024
                );
            }
            weather.refresh_if_due(now);
            thread::sleep(WEATHER_PERIOD);
        })
        .map_err(Fault::new)?;
    Ok(())
}

/// Serves the SD card to a computer on the USB cable; see `tools/sd.py`.
fn start_maintenance(card: SdmmcCard) -> Result<(), Fault> {
    usb_console::listen()?;
    thread::Builder::new()
        .name("maintenance".into())
        .stack_size(16 * 1024)
        .spawn(move || {
            let mut console = MaintenanceConsole::new(card);
            let mut line = String::new();
            loop {
                line.clear();
                let replied = io::stdin()
                    .lock()
                    .read_line(&mut line)
                    .and_then(|_| console.on_line(&line, &mut io::stdout().lock()));
                if let Err(e) = replied {
                    log::warn!("maintenance: {e}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        })
        .map_err(Fault::new)?;
    Ok(())
}

fn run_ui(mut controls: Controls, mut display: Uc8253, domain: Domain) {
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
        Box::new(SystemScreen::new(domain.foreground.clone(), domain.settings)),
        Box::new(WeatherScreen::new(domain.weather)),
        Box::new(CounterScreen::new(domain.counter)),
        Box::new(EchoScreen::default()),
        Box::new(PingScreen::new(domain.ping)),
    ];
    let Some(mut shell) = Shell::new(domain.foreground, screens) else {
        log::error!("ui: no app to show");
        return;
    };
    let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
    let mut frame = Frame::blank();
    let started = Instant::now();

    loop {
        let sample = controls.sample();
        if sample.is_touch() {
            domain.lighting.touched(Instant::now());
        }
        let changed = shell.on_sample(&sample, started.elapsed());
        if changed || shell.is_outdated() {
            let _ = frame.clear(BinaryColor::Off);
            let redraw = match shell.draw(&mut frame, visible) {
                ScreenChange::NewScreen => Redraw::Whole,
                ScreenChange::SameScreen => Redraw::Changes,
            };
            if let Err(fault) = display.show(&frame, redraw) {
                log::warn!("ui: {fault}");
            }
        }
        thread::sleep(CONTROLS_PERIOD);
    }
}
