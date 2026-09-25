//! Shows the screen of the app in front under its title, and opens the switcher over it
//! when asked. Which app is in front is the domain's; the shell only follows it.

use core::time::Duration;

use domain::apps::{App, Foreground};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};

use crate::app_screen::AppScreen;
use crate::controls::{Control, ControlsSample, Input};
use crate::gestures::{Gesture, Gestures};
use crate::switcher::Switcher;
use crate::text::{self, TITLE};

const MARGIN: i32 = 12;
const TITLE_RULE_GAP: i32 = 4;
const BODY_GAP: i32 = 10;
const SWITCHER_TITLE: &str = "Apps";

/// How what was just drawn relates to what was drawn before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenChange {
    /// Another screen came to the front: worth a clean redraw.
    NewScreen,
    /// The same screen shows something else.
    SameScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Showing {
    App(App),
    Switcher,
}

pub struct Shell<D> {
    gestures: Gestures,
    foreground: Foreground,
    screens: Vec<Box<dyn AppScreen<D>>>,
    switcher: Option<Switcher>,
    drawn: Option<Showing>,
}

impl<D: DrawTarget<Color = BinaryColor>> Shell<D> {
    /// `None` without any screen.
    pub fn new(foreground: Foreground, screens: Vec<Box<dyn AppScreen<D>>>) -> Option<Self> {
        (!screens.is_empty()).then_some(Self {
            gestures: Gestures::default(),
            foreground,
            screens,
            switcher: None,
            drawn: None,
        })
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

    /// The glass shows another screen than the one in front: something brought an app
    /// forward, or nothing has been drawn yet.
    pub fn is_outdated(&self) -> bool {
        self.drawn != Some(self.showing())
    }

    pub fn draw(&mut self, target: &mut D, area: Rectangle) -> ScreenChange {
        let inner = area.offset(-MARGIN);
        let rule_y = inner.top_left.y + TITLE.character_size.height as i32 + TITLE_RULE_GAP;
        let body_top = rule_y + BODY_GAP;
        let body_height = inner.top_left.y + inner.size.height as i32 - body_top;
        let body = Rectangle::new(Point::new(inner.top_left.x, body_top), Size::new(inner.size.width, body_height.max(0) as u32));

        let showing = self.showing();
        let title = match showing {
            Showing::App(app) => self.screen(app).map_or("", |s| s.title()),
            Showing::Switcher => SWITCHER_TITLE,
        };
        text::write(target, title, inner.top_left, inner.size.width, &TITLE);
        let rule_end = inner.top_left.x + inner.size.width as i32 - 1;
        let _ = Line::new(Point::new(inner.top_left.x, rule_y), Point::new(rule_end, rule_y))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(target);

        match (showing, self.switcher) {
            (Showing::Switcher, Some(switcher)) => {
                let titles: Vec<&str> = self.screens.iter().map(|s| s.title()).collect();
                switcher.draw(target, body, &titles);
            }
            (Showing::App(app), _) => {
                if let Some(screen) = self.screen(app) {
                    screen.draw(target, body);
                }
            }
            (Showing::Switcher, None) => {}
        }

        let change = if self.drawn == Some(showing) { ScreenChange::SameScreen } else { ScreenChange::NewScreen };
        self.drawn = Some(showing);
        change
    }

    fn follow(&mut self, gesture: Gesture) -> bool {
        let Some(mut switcher) = self.switcher else {
            return match gesture {
                Gesture::Switch => {
                    let front = self.position(self.foreground.app()).unwrap_or(0);
                    self.switcher = Switcher::open(front, self.screens.len());
                    true
                }
                Gesture::Input(input) => self.deliver(input),
            };
        };
        match gesture {
            Gesture::Input(Input::Turn(detents)) => {
                let moved = switcher.turn(detents);
                self.switcher = Some(switcher);
                moved
            }
            Gesture::Input(Input::Press(Control::Wheel)) => {
                if let Some(chosen) = self.screens.get(switcher.highlighted()) {
                    self.foreground.bring_to_front(chosen.app());
                }
                self.switcher = None;
                true
            }
            Gesture::Input(Input::Press(Control::Long)) | Gesture::Switch => {
                self.switcher = None;
                true
            }
            Gesture::Input(Input::Press(Control::Yellow)) => false,
        }
    }

    fn deliver(&mut self, input: Input) -> bool {
        let front = self.foreground.app();
        let Some(screen) = self.screens.iter_mut().find(|s| s.app() == front) else {
            return false;
        };
        screen.on_input(input);
        true
    }

    fn showing(&self) -> Showing {
        match self.switcher {
            Some(_) => Showing::Switcher,
            None => Showing::App(self.foreground.app()),
        }
    }

    fn position(&self, app: App) -> Option<usize> {
        self.screens.iter().position(|s| s.app() == app)
    }

    fn screen(&self, app: App) -> Option<&dyn AppScreen<D>> {
        self.screens.iter().find(|s| s.app() == app).map(|s| s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use domain::counter::Counter;
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::apps::{CounterScreen, EchoScreen};
    use crate::controls::ButtonSample;
    use crate::gestures::HOLD_TO_SWITCH;

    /// Remembers every input it was given.
    struct Recorder(App, Rc<RefCell<Vec<Input>>>);

    impl AppScreen<Frame> for Recorder {
        fn app(&self) -> App {
            self.0
        }
        fn title(&self) -> &str {
            "Recorder"
        }
        fn on_input(&mut self, input: Input) {
            self.1.borrow_mut().push(input);
        }
        fn draw(&self, _: &mut Frame, _: Rectangle) {}
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

    /// Opens the switcher, turns to the `index`-th screen and opens it.
    fn choose(shell: &mut Shell<Frame>, index: i32, at: Duration) {
        hold_long(shell, at);
        let from = shell.switcher.map_or(0, |s| s.highlighted() as i32);
        shell.on_sample(&turn(index - from), at);
        shell.on_sample(&press(Control::Wheel), at);
    }

    fn visible() -> Rectangle {
        Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()))
    }

    fn render(shell: &mut Shell<Frame>) -> (Frame, ScreenChange) {
        let mut frame = Frame::blank();
        let change = shell.draw(&mut frame, visible());
        (frame, change)
    }

    fn counter_and_echo(foreground: &Foreground) -> Shell<Frame> {
        let screens: Vec<Box<dyn AppScreen<Frame>>> =
            vec![Box::new(CounterScreen::new(Counter::new())), Box::new(EchoScreen::default())];
        Shell::new(foreground.clone(), screens).unwrap()
    }

    #[test]
    fn inputs_reach_only_the_app_in_front_and_never_the_switching() {
        let (counter, echo) = (Rc::default(), Rc::default());
        let screens: Vec<Box<dyn AppScreen<Frame>>> =
            vec![Box::new(Recorder(App::Counter, Rc::clone(&counter))), Box::new(Recorder(App::Echo, Rc::clone(&echo)))];
        let mut shell = Shell::new(Foreground::new(App::Counter), screens).unwrap();

        shell.on_sample(&turn(5), Duration::ZERO);
        choose(&mut shell, 1, Duration::from_secs(10));
        shell.on_sample(&press(Control::Yellow), Duration::from_secs(20));

        assert_eq!(*counter.borrow(), [Input::Turn(5)]);
        assert_eq!(*echo.borrow(), [Input::Press(Control::Yellow)]);
    }

    #[test]
    fn choosing_in_the_switcher_brings_the_app_to_the_front_of_the_domain() {
        let foreground = Foreground::new(App::Counter);
        let mut shell = counter_and_echo(&foreground);
        choose(&mut shell, 1, Duration::ZERO);
        assert_eq!(foreground.app(), App::Echo);
    }

    #[test]
    fn leaving_the_switcher_without_choosing_changes_nothing_in_the_domain() {
        let foreground = Foreground::new(App::Counter);
        let mut shell = counter_and_echo(&foreground);
        hold_long(&mut shell, Duration::ZERO);
        shell.on_sample(&turn(1), Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Counter, "turning the dot is not choosing");
        shell.on_sample(&press(Control::Long), Duration::from_secs(10));
        assert_eq!(foreground.app(), App::Counter);
        assert!(shell.switcher.is_none());
    }

    #[test]
    fn a_short_long_press_goes_to_the_app_not_the_switcher() {
        let mut shell = counter_and_echo(&Foreground::new(App::Echo));
        assert!(shell.on_sample(&press(Control::Long), Duration::ZERO));
        assert!(shell.switcher.is_none());
    }

    #[test]
    fn a_new_screen_in_front_is_a_new_screen_and_using_it_is_the_same_screen() {
        let mut shell = counter_and_echo(&Foreground::new(App::Counter));
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
        let screens: Vec<Box<dyn AppScreen<Frame>>> = vec![Box::new(CounterScreen::new(counter.clone()))];
        let mut shell = Shell::new(Foreground::new(App::Counter), screens).unwrap();
        let (before, _) = render(&mut shell);
        counter.add(7);
        assert!(render(&mut shell).0 != before);
    }

    #[test]
    fn nothing_is_drawn_outside_the_area_given() {
        let mut shell = counter_and_echo(&Foreground::new(App::Counter));
        let (app, _) = render(&mut shell);
        hold_long(&mut shell, Duration::ZERO);
        let (switcher, _) = render(&mut shell);
        for frame in [app, switcher] {
            for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
                }
            }
        }
    }
}
