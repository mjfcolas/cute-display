use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use ui::controls::Input;
use ui::{Describe, Description, DrawWithin, Screen};

/// Takes no input and draws nothing.
pub(crate) struct StubScreen;

pub(crate) struct Blank;

impl DrawWithin for Blank {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, _: &mut D, _: Rectangle) {}
}

impl Describe for Blank {
    fn describe(&self) -> Description {
        Description::default()
    }
}

impl Screen for StubScreen {
    type UiState = Blank;

    fn on_input(&mut self, _: Input) {}

    fn ui_state(&self) -> Blank {
        Blank
    }
}
