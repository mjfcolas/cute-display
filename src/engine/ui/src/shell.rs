use core::time::Duration;

use domain::apps::{AppId, Foreground};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::app_screen::HostedScreen;
use crate::controls::{Button, ControlsSample, Input};
use crate::description::Description;
use crate::gestures::{Gesture, Gestures};

const MARGIN: i32 = 8;

pub struct Hosted<D> {
    pub app: AppId,
    pub screen: Box<dyn HostedScreen<D>>,
}

pub struct Shell<D> {
    gestures: Gestures,
    foreground: Foreground,
    screens: Vec<Hosted<D>>,
    entered: Option<AppId>,
    drawn: Option<(AppId, u64)>,
}

impl<D: DrawTarget<Color = BinaryColor>> Shell<D> {
    pub fn new(foreground: Foreground, screens: Vec<Hosted<D>>) -> Option<Self> {
        (!screens.is_empty()).then_some(Self { gestures: Gestures::default(), foreground, screens, entered: None, drawn: None })
    }

    pub fn on_sample(&mut self, sample: &ControlsSample, now: Duration) -> bool {
        let mut touched = false;
        for gesture in self.gestures.interpret(sample, now) {
            touched |= self.follow(gesture);
        }
        touched
    }

    pub fn is_outdated(&self) -> bool {
        let front = self.foreground.app();
        self.drawn != Some((front, self.version_of(front)))
    }

    pub fn draw(&mut self, target: &mut D, area: Rectangle) -> Description {
        let front = self.enter_front();
        // Read before drawing: a change another thread makes meanwhile is then drawn next.
        let version = self.version_of(front);
        let mut description = Description::default();
        description.say("front", front.name());
        if let Some(hosted) = self.hosted(front) {
            description = description.followed_by(hosted.screen.draw(target, area.offset(-MARGIN)));
        }
        self.drawn = Some((front, version));
        description
    }

    fn follow(&mut self, gesture: Gesture) -> bool {
        match gesture {
            Gesture::System if self.foreground.app() == AppId::SYSTEM => return self.deliver(Input::Press(Button::Long)),
            Gesture::System => self.foreground.open_system(),
            Gesture::Input(input) => return self.deliver(input),
        }
        true
    }

    fn deliver(&mut self, input: Input) -> bool {
        let front = self.enter_front();
        let Some(hosted) = self.hosted_mut(front) else {
            return false;
        };
        hosted.screen.on_input(input);
        true
    }

    fn version_of(&self, app: AppId) -> u64 {
        self.hosted(app).map_or(0, |hosted| hosted.screen.version())
    }

    fn hosted(&self, app: AppId) -> Option<&Hosted<D>> {
        self.screens.iter().find(|hosted| hosted.app == app)
    }

    fn hosted_mut(&mut self, app: AppId) -> Option<&mut Hosted<D>> {
        self.screens.iter_mut().find(|hosted| hosted.app == app)
    }

    /// Tells a screen it came to the front before it is given anything else, and the one
    /// that was there that it left.
    fn enter_front(&mut self) -> AppId {
        let front = self.foreground.app();
        if self.entered != Some(front) {
            if let Some(hosted) = self.entered.and_then(|left| self.hosted_mut(left)) {
                hosted.screen.left();
            }
            if let Some(hosted) = self.hosted_mut(front) {
                hosted.screen.entered();
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

    use domain::settings::Settings;
    use domain_testing::settings::StubSettingsStore;
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::app_screen::{Describe, DrawWithin, Screen};
    use crate::system::{OfferedApp, SystemScreen};
    use crate::controls::{Button, ButtonSample};
    use crate::text::{self, BODY};

    const WEATHER: AppId = AppId::new("weather");
    const RADAR: AppId = AppId::new("radar");

    #[derive(Clone)]
    struct StubScreen {
        app: AppId,
        inputs: Rc<RefCell<Vec<Input>>>,
        visits: Rc<RefCell<Vec<&'static str>>>,
        text: Rc<RefCell<String>>,
        version: Rc<RefCell<u64>>,
    }

    impl StubScreen {
        fn new(app: AppId) -> Self {
            Self { app, inputs: Rc::default(), visits: Rc::default(), text: Rc::new(RefCell::new("probe".into())), version: Rc::default() }
        }
    }

    struct StubText(String);

    impl DrawWithin for StubText {
        fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
            text::write(target, &self.0, area.top_left, area.size.width, &BODY);
        }
    }

    impl Describe for StubText {
        fn describe(&self) -> Description {
            let mut description = Description::default();
            description.say("text", &self.0);
            description
        }
    }

    impl Screen for StubScreen {
        type UiState = StubText;

        fn entered(&mut self) {
            self.visits.borrow_mut().push("entered");
        }
        fn left(&mut self) {
            self.visits.borrow_mut().push("left");
        }
        fn version(&self) -> u64 {
            *self.version.borrow()
        }
        fn on_input(&mut self, input: Input) {
            self.inputs.borrow_mut().push(input);
        }
        fn ui_state(&self) -> StubText {
            StubText(self.text.borrow().clone())
        }
    }

    fn turn(detents: i32) -> ControlsSample {
        ControlsSample { detents, ..Default::default() }
    }

    fn press(button: Button) -> ControlsSample {
        let pressed = ButtonSample { presses: 1, held: false };
        match button {
            Button::Yellow => ControlsSample { yellow: pressed, ..Default::default() },
            Button::Long => ControlsSample { long: pressed, ..Default::default() },
        }
    }

    fn click_wheel(shell: &mut Shell<Frame>, at: Duration) {
        shell.on_sample(&ControlsSample { wheel: ButtonSample { presses: 1, held: false }, ..Default::default() }, at);
    }

    fn render(shell: &mut Shell<Frame>) -> Frame {
        let mut frame = Frame::blank();
        let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
        shell.draw(&mut frame, visible);
        frame
    }

    fn shell(foreground: &Foreground) -> (Shell<Frame>, StubScreen, StubScreen) {
        let (weather, radar) = (StubScreen::new(WEATHER), StubScreen::new(RADAR));
        let offered = [&weather, &radar].map(|probe| OfferedApp { app: probe.app, title: probe.app.name() }).into();
        let screens: Vec<Hosted<Frame>> = vec![
            Hosted { app: WEATHER, screen: Box::new(weather.clone()) },
            Hosted { app: RADAR, screen: Box::new(radar.clone()) },
            Hosted {
                app: AppId::SYSTEM,
                screen: Box::new(SystemScreen::new(foreground.clone(), Settings::load(Box::new(StubSettingsStore)), "2026.9.0", offered)),
            },
        ];
        (Shell::new(foreground.clone(), screens).unwrap(), weather, radar)
    }

    #[test]
    fn a_click_of_the_wheel_opens_the_system_app_and_there_confirms_like_the_long_button() {
        let foreground = Foreground::new(RADAR);
        let (mut shell, _, _) = shell(&foreground);
        click_wheel(&mut shell, Duration::ZERO);
        assert_eq!(foreground.app(), AppId::SYSTEM);
        shell.on_sample(&turn(-1), Duration::from_secs(5));
        click_wheel(&mut shell, Duration::from_secs(10));
        assert_eq!(foreground.app(), WEATHER, "the app under the dot, not the one it was opened from");
    }

    #[test]
    fn inputs_reach_only_the_app_in_front_and_never_the_wheels_click() {
        let foreground = Foreground::new(WEATHER);
        let (mut shell, weather, radar) = shell(&foreground);

        shell.on_sample(&turn(5), Duration::ZERO);
        click_wheel(&mut shell, Duration::from_secs(10));
        shell.on_sample(&turn(1), Duration::from_secs(20));
        shell.on_sample(&press(Button::Long), Duration::from_secs(20));
        assert_eq!(foreground.app(), RADAR);
        shell.on_sample(&press(Button::Yellow), Duration::from_secs(30));

        assert_eq!(*weather.inputs.borrow(), [Input::Turn(5)]);
        assert_eq!(*radar.inputs.borrow(), [Input::Press(Button::Yellow)]);
    }

    #[test]
    fn a_screen_is_told_it_came_to_the_front_before_its_first_input() {
        let foreground = Foreground::new(RADAR);
        let (mut shell, _, _) = shell(&foreground);
        click_wheel(&mut shell, Duration::ZERO);
        shell.on_sample(&press(Button::Long), Duration::from_secs(10));
        assert_eq!(foreground.app(), RADAR, "the dot started on Radar, where it was opened from");
    }

    #[test]
    fn a_screen_is_told_it_left_the_front_when_another_comes() {
        let foreground = Foreground::new(WEATHER);
        let (mut shell, weather, radar) = shell(&foreground);
        render(&mut shell);
        foreground.bring_to_front(RADAR);
        render(&mut shell);
        assert_eq!(*weather.visits.borrow(), ["entered", "left"]);
        assert_eq!(*radar.visits.borrow(), ["entered"]);
    }

    #[test]
    fn a_press_of_the_long_button_goes_to_the_app() {
        let foreground = Foreground::new(RADAR);
        let (mut shell, _, radar) = shell(&foreground);
        assert!(shell.on_sample(&press(Button::Long), Duration::ZERO));
        assert_eq!(foreground.app(), RADAR);
        assert_eq!(*radar.inputs.borrow(), [Input::Press(Button::Long)]);
    }

    #[test]
    fn drawing_brings_the_glass_up_to_date_until_another_app_comes_to_the_front() {
        let foreground = Foreground::new(WEATHER);
        let (mut shell, _, _) = shell(&foreground);
        assert!(shell.is_outdated(), "nothing drawn yet");
        render(&mut shell);
        assert!(!shell.is_outdated());

        click_wheel(&mut shell, Duration::from_secs(10));
        assert!(shell.is_outdated());
        render(&mut shell);
        assert!(!shell.is_outdated());
    }

    #[test]
    fn an_app_brought_forward_by_the_domain_outdates_the_glass() {
        let foreground = Foreground::new(WEATHER);
        let (mut shell, _, _) = shell(&foreground);
        render(&mut shell);
        foreground.bring_to_front(RADAR);
        assert!(shell.is_outdated());
    }

    #[test]
    fn an_app_that_changed_on_its_own_outdates_the_glass() {
        let (mut shell, weather, _) = shell(&Foreground::new(WEATHER));
        let before = render(&mut shell);
        assert!(!shell.is_outdated());
        *weather.text.borrow_mut() = "changed".into();
        *weather.version.borrow_mut() = 2;
        assert!(shell.is_outdated());
        let after = render(&mut shell);
        assert!(after != before);
        assert!(!shell.is_outdated());
    }

    #[test]
    fn nothing_is_drawn_outside_the_area_given() {
        let foreground = Foreground::new(WEATHER);
        let (mut shell, weather, _) = shell(&foreground);
        *weather.text.borrow_mut() = "a line far too long to fit on the glass at all, however small".into();
        let app = render(&mut shell);
        foreground.open_system();
        let system = render(&mut shell);
        for frame in [app, system] {
            for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn what_is_drawn_is_said_the_app_in_front_first() {
        let foreground = Foreground::new(RADAR);
        let (mut shell, _, radar) = shell(&foreground);
        *radar.text.borrow_mut() = "12 aircraft".into();
        let mut frame = Frame::blank();
        let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
        assert_eq!(shell.draw(&mut frame, visible).text(), ["front radar", "text 12 aircraft"]);
    }

    /// Its version moves on while it is drawn, as when another thread changes what it shows.
    struct StubChangingScreen(Rc<RefCell<u64>>);

    impl Screen for StubChangingScreen {
        type UiState = StubText;

        fn version(&self) -> u64 {
            *self.0.borrow()
        }
        fn on_input(&mut self, _: Input) {}
        fn ui_state(&self) -> StubText {
            *self.0.borrow_mut() += 1;
            StubText(String::new())
        }
    }

    #[test]
    fn a_change_made_while_the_screen_is_drawn_is_drawn_next() {
        let screens = vec![Hosted { app: WEATHER, screen: Box::new(StubChangingScreen(Rc::default())) as Box<dyn HostedScreen<Frame>> }];
        let mut shell = Shell::new(Foreground::new(WEATHER), screens).unwrap();
        render(&mut shell);
        assert!(shell.is_outdated());
    }
}
