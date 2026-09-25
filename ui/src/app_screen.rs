use domain::apps::App;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::controls::Input;

/// What an app looks like, and what its controls mean. It turns inputs into intents on
/// the domain and draws what the domain says; holding the long button never reaches it.
pub trait AppScreen<D: DrawTarget<Color = BinaryColor>> {
    fn app(&self) -> App;
    /// The app just came to the front.
    fn entered(&mut self) {}
    /// Changes whenever what the screen shows changed without an input: the screen is
    /// redrawn then. Screens that only change on input keep the default.
    fn version(&self) -> u64 {
        0
    }
    fn on_input(&mut self, input: Input);
    /// Draws within `area`, on paper.
    fn draw(&self, target: &mut D, area: Rectangle);
}

/// What an app is called on the glass.
pub fn title(app: App) -> &'static str {
    match app {
        App::System => "System",
        App::Weather => "Weather",
        App::Radar => "Radar",
    }
}
