use std::thread;
use std::time::{Duration, Instant};

use domain::lighting::Lighting;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{EpaperDisplay, Frame, Redraw, HEIGHT, VISIBLE_WIDTH};
use hal::input::{PushButton, RotaryEncoder};
use ui::apps::{AlarmScreen, RadarScreen, SystemScreen, WeatherScreen};
use ui::{AppScreen, Shell};

use crate::controls::Controls;
use crate::Domain;

const CONTROLS_PERIOD: Duration = Duration::from_millis(20);

/// The ui thread's devices: it reads the controls and draws on the panel.
pub(crate) struct Presentation<E, B, P> {
    pub controls: Controls<E, B>,
    pub panel: P,
}

impl<E: RotaryEncoder, B: PushButton, P: EpaperDisplay> Presentation<E, B, P> {
    pub fn run(mut self, domain: Domain) {
        let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
            Box::new(SystemScreen::new(domain.foreground.clone(), domain.settings, crate::VERSION)),
            Box::new(AlarmScreen::new(domain.alarm, domain.clock.clone(), domain.weather.clone())),
            Box::new(WeatherScreen::new(domain.weather, domain.clock)),
            Box::new(RadarScreen::new(domain.radar)),
        ];
        let Some(mut shell) = Shell::new(domain.foreground, screens) else {
            log::error!("ui: no app to show");
            return;
        };
        let mut frame = Frame::blank();
        let started = Instant::now();
        loop {
            self.tick(&mut shell, &domain.lighting, &mut frame, started.elapsed());
            thread::sleep(CONTROLS_PERIOD);
        }
    }

    /// Reads the controls once, and redraws the panel if the screen changed.
    fn tick(&mut self, shell: &mut Shell<Frame>, lighting: &Lighting, frame: &mut Frame, now: Duration) {
        let sample = self.controls.sample();
        if sample.is_touch() {
            lighting.touched(Instant::now());
        }
        let changed = shell.on_sample(&sample, now);
        if !changed && !shell.is_outdated() {
            return;
        }
        let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
        let _ = frame.clear(BinaryColor::Off);
        shell.draw(frame, visible);
        if let Err(fault) = self.panel.show(frame, Redraw::Changes) {
            log::warn!("ui: {fault}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use domain::apps::{App, Foreground};
    use domain::lighting::{Level, Light};
    use domain::settings::{Settings, SettingsRecord, SettingsStore};
    use hal::display::Refreshed;
    use hal::Fault;
    use ui::controls::Input;

    use super::*;

    #[derive(Clone, Default)]
    struct FakeWheel(Rc<Cell<i32>>);
    impl RotaryEncoder for FakeWheel {
        fn take_detents(&mut self) -> i32 {
            self.0.take()
        }
    }

    #[derive(Clone, Default)]
    struct FakeButton {
        presses: Rc<Cell<u32>>,
        held: Rc<Cell<bool>>,
    }
    impl PushButton for FakeButton {
        fn take_presses(&mut self) -> u32 {
            self.presses.take()
        }
        fn is_held(&self) -> bool {
            self.held.get()
        }
    }

    #[derive(Clone, Default)]
    struct FakePanel(Rc<Cell<Vec<Redraw>>>);
    impl FakePanel {
        fn take(&self) -> Vec<Redraw> {
            self.0.take()
        }
    }
    impl EpaperDisplay for FakePanel {
        fn show(&mut self, _: &Frame, redraw: Redraw) -> Result<Refreshed, Fault> {
            let mut shown = self.0.take();
            shown.push(redraw);
            self.0.set(shown);
            Ok(Refreshed::Nothing)
        }
    }

    struct Weather;
    impl AppScreen<Frame> for Weather {
        fn app(&self) -> App {
            App::Weather
        }
        fn on_input(&mut self, _: Input) {}
        fn draw(&self, _: &mut Frame, _: Rectangle) {}
    }

    struct Nowhere;
    impl SettingsStore for Nowhere {
        fn load(&mut self) -> Option<SettingsRecord> {
            None
        }
        fn save(&mut self, _: &SettingsRecord) {}
    }

    struct FakeLight(Arc<Mutex<Level>>);
    impl Light for FakeLight {
        fn shine(&mut self, level: Level) {
            *self.0.lock().unwrap() = level;
        }
    }

    #[test]
    fn every_redraw_asks_only_for_the_changes() {
        let (wheel, wheel_button, panel) = (FakeWheel::default(), FakeButton::default(), FakePanel::default());
        let mut presentation = Presentation {
            controls: Controls {
                wheel: wheel.clone(),
                wheel_button: wheel_button.clone(),
                yellow_button: FakeButton::default(),
                long_button: FakeButton::default(),
            },
            panel: panel.clone(),
        };
        let foreground = Foreground::new(App::Weather);
        let settings = Settings::load(Box::new(Nowhere));
        let screens: Vec<Box<dyn AppScreen<Frame>>> =
            vec![Box::new(Weather), Box::new(SystemScreen::new(foreground.clone(), settings.clone(), crate::VERSION))];
        let mut shell = Shell::new(foreground, screens).unwrap();
        let backlight = Arc::new(Mutex::new(Level::OFF));
        let lighting = Lighting::new(Box::new(FakeLight(backlight.clone())), Box::new(FakeLight(Arc::default())), settings);
        let mut frame = Frame::blank();
        let mut tick = |at: u64| presentation.tick(&mut shell, &lighting, &mut frame, Duration::from_secs(at));

        tick(0);
        assert_eq!(panel.take(), [Redraw::Changes]);
        tick(1);
        assert_eq!(panel.take(), []);
        assert_eq!(*backlight.lock().unwrap(), Level::OFF);

        wheel_button.presses.set(1);
        wheel_button.held.set(true);
        tick(2);
        assert_eq!(panel.take(), [Redraw::Changes], "a click of the wheel opens the system app");
        assert_ne!(*backlight.lock().unwrap(), Level::OFF, "a touch lights the screen");

        wheel_button.held.set(false);
        tick(4);
        wheel.0.set(1);
        tick(5);
        assert_eq!(panel.take(), [Redraw::Changes]);
    }
}
