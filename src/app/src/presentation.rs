use std::time::{Duration, Instant};

use domain::apps::{AppId, Foreground};
use domain::lighting::Lighting;
use domain::settings::Settings;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use hal::display::{EpaperDisplay, Frame, Redraw, HEIGHT, VISIBLE_WIDTH};
use hal::input::{PushButton, RotaryEncoder};
use hal::steady::SteadyClock;
use ui::system::{OfferedApp, SystemScreen};
use ui::{HostedScreen, Hosted, Shell};

use crate::controls::Controls;

const CONTROLS_PERIOD: Duration = Duration::from_millis(20);

pub(crate) struct AppOnScreen {
    pub offered: OfferedApp,
    pub screen: Box<dyn HostedScreen<Frame> + Send>,
}

pub(crate) struct Presentation<E, B, P, R, T> {
    pub controls: Controls<E, B>,
    pub panel: P,
    pub read_out: R,
    pub steady: T,
    /// Where the gestures count their time from.
    pub started: Instant,
}

impl<E: RotaryEncoder, B: PushButton, P: EpaperDisplay, R: FnMut(Vec<String>), T: SteadyClock> Presentation<E, B, P, R, T> {
    pub fn run(mut self, foreground: Foreground, settings: Settings, lighting: Lighting, apps: Vec<AppOnScreen>) {
        let offered = apps.iter().map(|app| app.offered).collect();
        let mut screens: Vec<Hosted<Frame>> = apps.into_iter().map(|app| Hosted { app: app.offered.app, screen: app.screen }).collect();
        let system = SystemScreen::new(foreground.clone(), settings, crate::image::VERSION, offered);
        screens.push(Hosted { app: AppId::SYSTEM, screen: Box::new(system) });
        let Some(mut shell) = Shell::new(foreground, screens) else {
            log::error!("ui: no app to show");
            return;
        };
        let mut frame = Frame::blank();
        loop {
            let now = self.steady.now();
            self.tick(&mut shell, &lighting, &mut frame, now);
            self.steady.sleep(CONTROLS_PERIOD);
        }
    }

    fn tick(&mut self, shell: &mut Shell<Frame>, lighting: &Lighting, frame: &mut Frame, now: Instant) {
        let sample = self.controls.sample();
        if sample.is_touch() {
            lighting.touched(now);
        }
        let changed = shell.on_sample(&sample, now.saturating_duration_since(self.started));
        if !changed && !shell.is_outdated() {
            return;
        }
        let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
        let _ = frame.clear(BinaryColor::Off);
        let description = shell.draw(frame, visible);
        match self.panel.show(frame, Redraw::Changes) {
            Ok(_) => (self.read_out)(description.text()),
            Err(fault) => log::warn!("ui: {fault}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::lighting::{Level, Light};
    use domain_testing::settings::StubSettingsStore;
    use hal::display::Refreshed;
    use hal::Fault;
    use hal_testing::display::StubPanel;
    use hal_testing::input::{FakeButton, FakeWheel};
    use hal_testing::steady::FakeSteadyClock;

    use super::*;
    use crate::stub_screen::StubScreen;

    struct StubLight(Arc<Mutex<Level>>);
    impl Light for StubLight {
        fn shine(&mut self, level: Level) {
            *self.0.lock().unwrap() = level;
        }
    }

    struct StubFailingPanel;
    impl EpaperDisplay for StubFailingPanel {
        fn show(&mut self, _: &Frame, _: Redraw) -> Result<Refreshed, Fault> {
            Err(Fault::new("the panel stayed busy"))
        }
    }

    fn weather_in_front(settings: &Settings) -> Shell<Frame> {
        let foreground = Foreground::new(AppId::new("weather"));
        let offered = vec![OfferedApp { app: AppId::new("weather"), title: "Weather" }];
        let system = SystemScreen::new(foreground.clone(), settings.clone(), crate::image::VERSION, offered);
        let screens = vec![
            Hosted { app: AppId::new("weather"), screen: Box::new(StubScreen) as Box<dyn HostedScreen<Frame>> },
            Hosted { app: AppId::SYSTEM, screen: Box::new(system) },
        ];
        Shell::new(foreground, screens).unwrap()
    }

    fn heard_after_showing_on(panel: impl EpaperDisplay) -> Option<Vec<String>> {
        let heard: Arc<Mutex<Option<Vec<String>>>> = Arc::default();
        let hears = heard.clone();
        let started = Instant::now();
        let mut presentation = Presentation {
            controls: Controls {
                wheel: FakeWheel::default(),
                wheel_button: FakeButton::default(),
                yellow_button: FakeButton::default(),
                long_button: FakeButton::default(),
            },
            panel,
            read_out: move |lines| *hears.lock().unwrap() = Some(lines),
            steady: FakeSteadyClock::default(),
            started,
        };
        let settings = Settings::load(Box::new(StubSettingsStore));
        let lighting = Lighting::new(Box::new(StubLight(Arc::default())), Box::new(StubLight(Arc::default())), settings.clone());
        presentation.tick(&mut weather_in_front(&settings), &lighting, &mut Frame::blank(), started);
        let lines = heard.lock().unwrap().clone();
        lines
    }

    #[test]
    fn what_is_shown_is_read_out_and_what_the_panel_did_not_show_is_not() {
        assert_eq!(heard_after_showing_on(StubPanel::default()), Some(vec!["front weather".to_owned()]));
        assert_eq!(heard_after_showing_on(StubFailingPanel), None);
    }

    #[test]
    fn every_redraw_asks_only_for_the_changes() {
        let (wheel, wheel_button, panel) = (FakeWheel::default(), FakeButton::default(), StubPanel::default());
        let mut presentation = Presentation {
            controls: Controls {
                wheel: wheel.clone(),
                wheel_button: wheel_button.clone(),
                yellow_button: FakeButton::default(),
                long_button: FakeButton::default(),
            },
            panel: panel.clone(),
            read_out: |_| {},
            steady: FakeSteadyClock::default(),
            started: Instant::now(),
        };
        let settings = Settings::load(Box::new(StubSettingsStore));
        let mut shell = weather_in_front(&settings);
        let backlight = Arc::new(Mutex::new(Level::OFF));
        let lighting = Lighting::new(Box::new(StubLight(backlight.clone())), Box::new(StubLight(Arc::default())), settings);
        let mut frame = Frame::blank();
        let started = presentation.started;
        let mut tick = |at: u64| presentation.tick(&mut shell, &lighting, &mut frame, started + Duration::from_secs(at));

        tick(0);
        assert_eq!(panel.take_redraws(), [Redraw::Changes]);
        tick(1);
        assert_eq!(panel.take_redraws(), []);
        assert_eq!(*backlight.lock().unwrap(), Level::OFF);

        wheel_button.press();
        wheel_button.set_held(true);
        tick(2);
        assert_eq!(panel.take_redraws(), [Redraw::Changes], "a click of the wheel opens the system app");
        assert_ne!(*backlight.lock().unwrap(), Level::OFF, "a touch lights the screen");

        wheel_button.set_held(false);
        tick(4);
        wheel.turn(1);
        tick(5);
        assert_eq!(panel.take_redraws(), [Redraw::Changes]);
    }
}
