use domain::apps::App;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::draw_lines;
use crate::app_screen::AppScreen;
use crate::controls::{Control, Input};

/// Shows what reached it, to see which controls an app is given. A diagnostic: it has
/// nothing to say to the domain, so its state is its own.
#[derive(Default)]
pub struct EchoScreen {
    last: Option<Input>,
    received: u32,
}

fn describe(input: Input) -> String {
    match input {
        Input::Turn(detents) => format!("wheel turned {detents:+}"),
        Input::Press(Control::Wheel) => "wheel pressed".into(),
        Input::Press(Control::Yellow) => "yellow pressed".into(),
        Input::Press(Control::Long) => "long pressed".into(),
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for EchoScreen {
    fn app(&self) -> App {
        App::Echo
    }


    fn on_input(&mut self, input: Input) {
        self.last = Some(input);
        self.received = self.received.saturating_add(1);
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        let last = self.last.map_or_else(|| "nothing yet".into(), describe);
        let received = format!("{} input(s)", self.received);
        draw_lines(target, area, &[&last, &received], "shows every input it receives");
    }
}
