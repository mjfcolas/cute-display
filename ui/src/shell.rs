//! Shows the screen of the app in front, under its title. Which app is in front is the
//! domain's; the shell follows it, and turns the system gesture into opening or closing
//! the system app.

use core::time::Duration;

use domain::apps::{App, Foreground};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};

use crate::app_screen::{title, AppScreen};
use crate::controls::{ControlsSample, Input};
use crate::gestures::{Gesture, Gestures};
use crate::text::{self, TITLE};

const MARGIN: i32 = 12;
const TITLE_RULE_GAP: i32 = 4;
const BODY_GAP: i32 = 10;

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
    drawn: Option<App>,
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

    /// The glass shows another app than the one in front: something brought an app
    /// forward, or nothing has been drawn yet.
    pub fn is_outdated(&self) -> bool {
        self.drawn != Some(self.foreground.app())
    }

    pub fn draw(&mut self, target: &mut D, area: Rectangle) -> ScreenChange {
        let front = self.enter_front();
        let inner = area.offset(-MARGIN);
        let rule_y = inner.top_left.y + TITLE.character_size.height as i32 + TITLE_RULE_GAP;
        let body_top = rule_y + BODY_GAP;
        let body_height = inner.top_left.y + inner.size.height as i32 - body_top;
        let body = Rectangle::new(Point::new(inner.top_left.x, body_top), Size::new(inner.size.width, body_height.max(0) as u32));

        text::write(target, title(front), inner.top_left, inner.size.width, &TITLE);
        let rule_end = inner.top_left.x + inner.size.width as i32 - 1;
        let _ = Line::new(Point::new(inner.top_left.x, rule_y), Point::new(rule_end, rule_y))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(target);
        if let Some(screen) = self.screens.iter().find(|s| s.app() == front) {
            screen.draw(target, body);
        }

        let change = if self.drawn == Some(front) { ScreenChange::SameScreen } else { ScreenChange::NewScreen };
        self.drawn = Some(front);
        change
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

    use domain::counter::Counter;
    use domain::settings::{Settings, SettingsRecord, SettingsStore};
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::apps::{CounterScreen, EchoScreen, SystemScreen};
    use crate::controls::{ButtonSample, Control};
    use crate::gestures::HOLD_TO_SWITCH;

    /// Remembers every input it was given.
    struct Recorder(App, Rc<RefCell<Vec<Input>>>);

    impl AppScreen<Frame> for Recorder {
        fn app(&self) -> App {
            self.0
        }
        fn on_input(&mut self, input: Input) {
            self.1.borrow_mut().push(input);
        }
        fn draw(&self, _: &mut Frame, _: Rectangle) {}
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

    fn with_system(foreground: &Foreground, mut screens: Vec<Box<dyn AppScreen<Frame>>>) -> Shell<Frame> {
        let settings = Settings::load(Box::new(Nowhere));
        screens.push(Box::new(SystemScreen::new(foreground.clone(), settings)));
        Shell::new(foreground.clone(), screens).unwrap()
    }

    fn counter_and_echo(foreground: &Foreground) -> Shell<Frame> {
        with_system(foreground, vec![Box::new(CounterScreen::new(Counter::new())), Box::new(EchoScreen::default())])
    }

    #[test]
    fn holding_the_long_button_opens_the_system_app_and_again_closes_it() {
        let foreground = Foreground::new(App::Echo);
        let mut shell = counter_and_echo(&foreground);
        hold_long(&mut shell, Duration::ZERO);
        assert_eq!(foreground.app(), App::System);
        hold_long(&mut shell, Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Echo);
    }

    #[test]
    fn inputs_reach_only_the_app_in_front_and_never_the_system_gesture() {
        let (counter, echo) = (Rc::default(), Rc::default());
        let foreground = Foreground::new(App::Counter);
        let mut shell = with_system(
            &foreground,
            vec![Box::new(Recorder(App::Counter, Rc::clone(&counter))), Box::new(Recorder(App::Echo, Rc::clone(&echo)))],
        );

        shell.on_sample(&turn(5), Duration::ZERO);
        hold_long(&mut shell, Duration::from_secs(10));
        shell.on_sample(&turn(1), Duration::from_secs(20));
        shell.on_sample(&press(Control::Wheel), Duration::from_secs(20));
        assert_eq!(foreground.app(), App::Echo);
        shell.on_sample(&press(Control::Yellow), Duration::from_secs(30));

        assert_eq!(*counter.borrow(), [Input::Turn(5)]);
        assert_eq!(*echo.borrow(), [Input::Press(Control::Yellow)]);
    }

    #[test]
    fn a_screen_is_told_it_came_to_the_front_before_its_first_input() {
        let foreground = Foreground::new(App::Echo);
        let mut shell = counter_and_echo(&foreground);
        hold_long(&mut shell, Duration::ZERO);
        shell.on_sample(&press(Control::Wheel), Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Echo, "the dot started on Echo, where it was opened from");
    }

    #[test]
    fn a_short_long_press_goes_to_the_app_not_the_system() {
        let foreground = Foreground::new(App::Echo);
        let mut shell = counter_and_echo(&foreground);
        assert!(shell.on_sample(&press(Control::Long), Duration::ZERO));
        assert_eq!(foreground.app(), App::Echo);
    }

    #[test]
    fn a_new_app_in_front_is_a_new_screen_and_using_it_is_the_same_screen() {
        let foreground = Foreground::new(App::Counter);
        let mut shell = counter_and_echo(&foreground);
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
        let foreground = Foreground::new(App::Counter);
        let mut shell = counter_and_echo(&foreground);
        render(&mut shell);
        foreground.bring_to_front(App::Echo);
        assert!(shell.is_outdated());
        assert_eq!(render(&mut shell).1, ScreenChange::NewScreen);
    }

    #[test]
    fn an_app_is_drawn_from_the_domain_whoever_changed_it() {
        let counter = Counter::new();
        let foreground = Foreground::new(App::Counter);
        let mut shell = with_system(&foreground, vec![Box::new(CounterScreen::new(counter.clone()))]);
        let (before, _) = render(&mut shell);
        counter.add(7);
        assert!(render(&mut shell).0 != before);
    }

    #[test]
    fn nothing_is_drawn_outside_the_area_given() {
        let foreground = Foreground::new(App::Counter);
        let mut shell = counter_and_echo(&foreground);
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
