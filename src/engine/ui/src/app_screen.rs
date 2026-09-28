use std::sync::Arc;

use domain::apps::{AppId, AppService, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::controls::Input;

pub trait AppScreen<D: DrawTarget<Color = BinaryColor>> {
    fn entered(&mut self) {}
    fn left(&mut self) {}
    /// Changes whenever what the screen shows changed without an input: the screen is
    /// redrawn then. Screens that only change on input keep the default.
    fn version(&self) -> u64 {
        0
    }
    fn on_input(&mut self, input: Input);
    /// Draws within `area`, on paper.
    fn draw(&self, target: &mut D, area: Rectangle);
}

pub struct InstalledApp<D> {
    pub service: Arc<dyn AppService>,
    pub screen: Box<dyn AppScreen<D> + Send>,
}

pub type Install<D> = fn(&dyn Services) -> InstalledApp<D>;

pub struct Installable<D> {
    pub id: AppId,
    pub title: &'static str,
    pub install: Install<D>,
}
