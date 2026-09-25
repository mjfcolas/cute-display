use domain::apps::App;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::controls::Input;

/// What an app looks like, and what its controls mean. It turns inputs into intents on
/// the domain and draws what the domain says; holding the long button never reaches it.
pub trait AppScreen<D: DrawTarget<Color = BinaryColor>> {
    fn app(&self) -> App;
    fn title(&self) -> &str;
    fn on_input(&mut self, input: Input);
    /// Draws within `area`, on paper.
    fn draw(&self, target: &mut D, area: Rectangle);
}
