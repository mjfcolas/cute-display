use std::thread;
use std::time::{Duration, Instant};

use domain::apps::{App, Foreground};
use domain::counter::Counter;
use domain::ping::Ping;
use drivers::button::Button;
use drivers::pcnt_encoder::PcntEncoder;
use drivers::uc8253::Uc8253;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::FreeRtos;
use hal::display::{EpaperDisplay, Frame, Redraw, HEIGHT, VISIBLE_WIDTH};
use hal::input::{PushButton, RotaryEncoder};
use hal::light::{Brightness, DimmableLight};
use hal::Fault;
use ui::apps::{CounterScreen, EchoScreen, PingScreen};
use ui::controls::{ButtonSample, ControlsSample};
use ui::{AppScreen, ScreenChange, Shell};

use firmware::board::Board;

const BUILD: &str = concat!("build ", env!("BUILD_TIME"), " @", env!("BUILD_GIT"));
const CONTROLS_PERIOD: Duration = Duration::from_millis(20);
/// Until the front light has an owner of its own, enough to read the glass in the dark.
const FRONT_LIGHT: Brightness = Brightness::percent(20);

/// Everything the domain is made of, shared by the threads that use it.
struct Domain {
    foreground: Foreground,
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

    let mut board = Board::bring_up()?;
    board.front_light.set_brightness(FRONT_LIGHT)?;
    let domain = Domain { foreground: Foreground::new(App::Counter), counter: Counter::new(), ping: Ping::new() };

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
        thread::sleep(Duration::from_secs(60));
    }
}

fn run_ui(mut controls: Controls, mut display: Uc8253, domain: Domain) {
    let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
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
        let touched = shell.on_sample(&controls.sample(), started.elapsed());
        if touched || shell.is_outdated() {
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
