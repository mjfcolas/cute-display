use std::sync::Arc;

use domain::apps::{AppId, AppService, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::controls::Input;

/// What an app looks like, and what its controls mean. It turns the controls into its
/// app's intents and draws its state.
pub trait AppScreen<D: DrawTarget<Color = BinaryColor>> {
    fn app(&self) -> AppId;
    /// What the app is called on the glass.
    fn title(&self) -> &'static str;
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

/// An app as the engine runs it: what runs whatever is on screen, and its screen.
pub struct InstalledApp<D> {
    pub service: Arc<dyn AppService>,
    pub screen: Box<dyn AppScreen<D> + Send>,
}

/// How an app installs itself on what the engine lends it.
pub type Install<D> = fn(&dyn Services) -> InstalledApp<D>;
