use domain::apps::App;
use domain::ping::Ping;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::draw_lines;
use crate::app_screen::AppScreen;
use crate::controls::{Control, Input};

/// Triggers Ping, and shows what the domain says about it.
pub struct PingScreen {
    ping: Ping,
}

impl PingScreen {
    pub fn new(ping: Ping) -> Self {
        Self { ping }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for PingScreen {
    fn app(&self) -> App {
        App::Ping
    }

    fn title(&self) -> &str {
        "Ping"
    }

    fn on_input(&mut self, input: Input) {
        if input == Input::Press(Control::Wheel) {
            self.ping.trigger();
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        let triggered = format!("Triggered {} time(s)", self.ping.state().times_triggered);
        draw_lines(target, area, &[&triggered], "press: trigger ping");
    }
}

#[cfg(test)]
mod tests {
    use hal::display::Frame;

    use super::*;

    fn draw(screen: &PingScreen) -> Frame {
        let mut frame = Frame::blank();
        AppScreen::<Frame>::draw(screen, &mut frame, Rectangle::new(Point::zero(), Size::new(300, 100)));
        frame
    }

    #[test]
    fn a_wheel_press_triggers_ping_and_nothing_else_does() {
        let ping = Ping::new();
        let mut screen = PingScreen::new(ping.clone());
        for input in [Input::Turn(3), Input::Press(Control::Yellow), Input::Press(Control::Long)] {
            AppScreen::<Frame>::on_input(&mut screen, input);
        }
        assert_eq!(ping.state().times_triggered, 0);
        AppScreen::<Frame>::on_input(&mut screen, Input::Press(Control::Wheel));
        assert_eq!(ping.state().times_triggered, 1);
    }

    #[test]
    fn it_shows_what_the_domain_says_even_when_changed_elsewhere() {
        let ping = Ping::new();
        let screen = PingScreen::new(ping.clone());
        let before = draw(&screen);
        ping.trigger();
        assert!(draw(&screen) != before);
    }
}
