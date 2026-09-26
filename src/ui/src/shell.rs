//! Shows the screen of the app in front, on the whole glass. Which app is in front is the
//! domain's; the shell follows it, and turns the system gesture into opening or closing
//! the system app.

use core::time::Duration;

use domain::apps::{App, Foreground};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::app_screen::AppScreen;
use crate::controls::{ControlsSample, Input};
use crate::gestures::{Gesture, Gestures};

const MARGIN: i32 = 8;

/// How what was just drawn relates to what was drawn before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenChange {
    /// Another app came to the front: worth a clean redraw.
    NewScreen,
    /// The same app shows something else.
    SameScreen,
}

pub struct Shell<D> {
    gestures: Gestures,
    foreground: Foreground,
    screens: Vec<Box<dyn AppScreen<D>>>,
    /// The app whose screen was last told it came to the front.
    entered: Option<App>,
    /// The app last drawn, and its version then.
    drawn: Option<(App, u64)>,
}

impl<D: DrawTarget<Color = BinaryColor>> Shell<D> {
    /// `None` without any screen.
    pub fn new(foreground: Foreground, screens: Vec<Box<dyn AppScreen<D>>>) -> Option<Self> {
        (!screens.is_empty()).then_some(Self { gestures: Gestures::default(), foreground, screens, entered: None, drawn: None })
    }

    /// Whether the controls did anything the glass should show. `now` is any monotonic
    /// time, as long as it is always the same clock.
    pub fn on_sample(&mut self, sample: &ControlsSample, now: Duration) -> bool {
        let mut touched = false;
        for gesture in self.gestures.interpret(sample, now) {
            touched |= self.follow(gesture);
        }
        touched
    }

    /// The glass no longer shows what it should: something brought another app forward,
    /// the app in front changed on its own, or nothing has been drawn yet.
    pub fn is_outdated(&self) -> bool {
        let front = self.foreground.app();
        self.drawn != Some((front, self.version_of(front)))
    }

    pub fn draw(&mut self, target: &mut D, area: Rectangle) -> ScreenChange {
        let front = self.enter_front();
        if let Some(screen) = self.screens.iter().find(|s| s.app() == front) {
            screen.draw(target, area.offset(-MARGIN));
        }

        let same_app = self.drawn.is_some_and(|(app, _)| app == front);
        self.drawn = Some((front, self.version_of(front)));
        if same_app { ScreenChange::SameScreen } else { ScreenChange::NewScreen }
    }

    fn follow(&mut self, gesture: Gesture) -> bool {
        match gesture {
            Gesture::Switch if self.foreground.app() == App::System => self.foreground.close_system(),
            Gesture::Switch => self.foreground.open_system(),
            Gesture::Input(input) => return self.deliver(input),
        }
        true
    }

    fn deliver(&mut self, input: Input) -> bool {
        let front = self.enter_front();
        let Some(screen) = self.screens.iter_mut().find(|s| s.app() == front) else {
            return false;
        };
        screen.on_input(input);
        true
    }

    fn version_of(&self, app: App) -> u64 {
        self.screens.iter().find(|s| s.app() == app).map_or(0, |s| s.version())
    }

    /// Tells a screen it came to the front before it is given anything else.
    fn enter_front(&mut self) -> App {
        let front = self.foreground.app();
        if self.entered != Some(front) {
            if let Some(screen) = self.screens.iter_mut().find(|s| s.app() == front) {
                screen.entered();
            }
            self.entered = Some(front);
        }
        front
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use domain::settings::{Settings, SettingsRecord, SettingsStore};
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::apps::SystemScreen;
    use crate::controls::{ButtonSample, Control};
    use crate::gestures::HOLD_TO_SWITCH;
    use crate::text::{self, BODY};

    /// Remembers every input it was given, and draws a line of text it may be given. Its
    /// clones share all of it, so a test keeps one while the shell holds another.
    #[derive(Clone)]
    struct Probe {
        app: App,
        inputs: Rc<RefCell<Vec<Input>>>,
        text: Rc<RefCell<String>>,
        version: Rc<RefCell<u64>>,
    }

    impl Probe {
        fn new(app: App) -> Self {
            Self { app, inputs: Rc::default(), text: Rc::new(RefCell::new("probe".into())), version: Rc::default() }
        }
    }

    impl AppScreen<Frame> for Probe {
        fn app(&self) -> App {
            self.app
        }
        fn version(&self) -> u64 {
            *self.version.borrow()
        }
        fn on_input(&mut self, input: Input) {
            self.inputs.borrow_mut().push(input);
        }
        fn draw(&self, target: &mut Frame, area: Rectangle) {
            text::write(target, &self.text.borrow(), area.top_left, area.size.width, &BODY);
        }
    }

    struct Nowhere;

    impl SettingsStore for Nowhere {
        fn load(&mut self) -> Option<SettingsRecord> {
            None
        }
        fn save(&mut self, _: &SettingsRecord) {}
    }

    fn turn(detents: i32) -> ControlsSample {
        ControlsSample { detents, ..Default::default() }
    }

    fn press(control: Control) -> ControlsSample {
        let pressed = ButtonSample { presses: 1, held: false };
        match control {
            Control::Wheel => ControlsSample { wheel: pressed, ..Default::default() },
            Control::Yellow => ControlsSample { yellow: pressed, ..Default::default() },
            Control::Long => ControlsSample { long: pressed, ..Default::default() },
        }
    }

    fn hold_long(shell: &mut Shell<Frame>, from: Duration) {
        let held = |presses| ControlsSample { long: ButtonSample { presses, held: true }, ..Default::default() };
        shell.on_sample(&held(1), from);
        shell.on_sample(&held(0), from + HOLD_TO_SWITCH);
        shell.on_sample(&ControlsSample::default(), from + HOLD_TO_SWITCH * 2);
    }

    fn render(shell: &mut Shell<Frame>) -> (Frame, ScreenChange) {
        let mut frame = Frame::blank();
        let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
        let change = shell.draw(&mut frame, visible);
        (frame, change)
    }

    /// The weather and radar apps as probes, and the real system app.
    fn shell(foreground: &Foreground) -> (Shell<Frame>, Probe, Probe) {
        let (weather, radar) = (Probe::new(App::Weather), Probe::new(App::Radar));
        let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![
            Box::new(weather.clone()),
            Box::new(radar.clone()),
            Box::new(SystemScreen::new(foreground.clone(), Settings::load(Box::new(Nowhere)))),
        ];
        (Shell::new(foreground.clone(), screens).unwrap(), weather, radar)
    }

    #[test]
    fn holding_the_long_button_opens_the_system_app_and_again_closes_it() {
        let foreground = Foreground::new(App::Radar);
        let (mut shell, _, _) = shell(&foreground);
        hold_long(&mut shell, Duration::ZERO);
        assert_eq!(foreground.app(), App::System);
        hold_long(&mut shell, Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Radar);
    }

    #[test]
    fn inputs_reach_only_the_app_in_front_and_never_the_system_gesture() {
        let foreground = Foreground::new(App::Weather);
        let (mut shell, weather, radar) = shell(&foreground);

        shell.on_sample(&turn(5), Duration::ZERO);
        hold_long(&mut shell, Duration::from_secs(10));
        shell.on_sample(&turn(1), Duration::from_secs(20));
        shell.on_sample(&press(Control::Wheel), Duration::from_secs(20));
        assert_eq!(foreground.app(), App::Radar);
        shell.on_sample(&press(Control::Yellow), Duration::from_secs(30));

        assert_eq!(*weather.inputs.borrow(), [Input::Turn(5)]);
        assert_eq!(*radar.inputs.borrow(), [Input::Press(Control::Yellow)]);
    }

    #[test]
    fn a_screen_is_told_it_came_to_the_front_before_its_first_input() {
        let foreground = Foreground::new(App::Radar);
        let (mut shell, _, _) = shell(&foreground);
        hold_long(&mut shell, Duration::ZERO);
        shell.on_sample(&press(Control::Wheel), Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Radar, "the dot started on Radar, where it was opened from");
    }

    #[test]
    fn a_short_long_press_goes_to_the_app_not_the_system() {
        let foreground = Foreground::new(App::Radar);
        let (mut shell, _, radar) = shell(&foreground);
        assert!(shell.on_sample(&press(Control::Long), Duration::ZERO));
        assert_eq!(foreground.app(), App::Radar);
        assert_eq!(*radar.inputs.borrow(), [Input::Press(Control::Long)]);
    }

    #[test]
    fn a_new_app_in_front_is_a_new_screen_and_using_it_is_the_same_screen() {
        let foreground = Foreground::new(App::Weather);
        let (mut shell, _, _) = shell(&foreground);
        assert!(shell.is_outdated(), "nothing drawn yet");
        assert_eq!(render(&mut shell).1, ScreenChange::NewScreen);
        assert!(!shell.is_outdated());

        shell.on_sample(&turn(1), Duration::ZERO);
        assert_eq!(render(&mut shell).1, ScreenChange::SameScreen);

        hold_long(&mut shell, Duration::from_secs(10));
        assert_eq!(render(&mut shell).1, ScreenChange::NewScreen);
    }

    #[test]
    fn an_app_brought_forward_by_the_domain_outdates_the_glass() {
        let foreground = Foreground::new(App::Weather);
        let (mut shell, _, _) = shell(&foreground);
        render(&mut shell);
        foreground.bring_to_front(App::Radar);
        assert!(shell.is_outdated());
        assert_eq!(render(&mut shell).1, ScreenChange::NewScreen);
    }

    #[test]
    fn an_app_that_changed_on_its_own_is_redrawn_as_the_same_screen() {
        let (mut shell, weather, _) = shell(&Foreground::new(App::Weather));
        let (before, _) = render(&mut shell);
        assert!(!shell.is_outdated());
        *weather.text.borrow_mut() = "changed".into();
        *weather.version.borrow_mut() = 2;
        assert!(shell.is_outdated());
        let (after, change) = render(&mut shell);
        assert_eq!(change, ScreenChange::SameScreen);
        assert!(after != before);
        assert!(!shell.is_outdated());
    }

    #[test]
    fn nothing_is_drawn_outside_the_area_given() {
        let foreground = Foreground::new(App::Weather);
        let (mut shell, weather, _) = shell(&foreground);
        *weather.text.borrow_mut() = "a line far too long to fit on the glass at all, however small".into();
        let (app, _) = render(&mut shell);
        foreground.open_system();
        let (system, _) = render(&mut shell);
        for frame in [app, system] {
            for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
                }
            }
        }
    }
}
