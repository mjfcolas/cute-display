use domain::apps::App;
use domain::counter::Counter;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::draw_lines;
use crate::app_screen::AppScreen;
use crate::controls::{Control, Input};

pub struct CounterScreen {
    counter: Counter,
}

impl CounterScreen {
    pub fn new(counter: Counter) -> Self {
        Self { counter }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for CounterScreen {
    fn app(&self) -> App {
        App::Counter
    }


    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => self.counter.add(detents),
            Input::Press(Control::Yellow) => self.counter.reset(),
            Input::Press(_) => {}
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        draw_lines(target, area, &[&format!("Count: {}", self.counter.count())], "wheel: count   yellow: reset");
    }
}

#[cfg(test)]
mod tests {
    use hal::display::Frame;

    use super::*;

    #[test]
    fn the_wheel_counts_and_the_yellow_button_resets() {
        let counter = Counter::new();
        let mut screen = CounterScreen::new(counter.clone());
        AppScreen::<Frame>::on_input(&mut screen, Input::Turn(4));
        AppScreen::<Frame>::on_input(&mut screen, Input::Turn(-1));
        assert_eq!(counter.count(), 3);
        AppScreen::<Frame>::on_input(&mut screen, Input::Press(Control::Yellow));
        assert_eq!(counter.count(), 0);
    }
}
