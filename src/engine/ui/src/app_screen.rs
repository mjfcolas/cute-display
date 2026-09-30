use std::sync::Arc;

use domain::apps::{AppId, AppService, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::controls::Input;
use crate::description::Description;

pub trait DrawWithin {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle);
}

pub trait Describe {
    fn describe(&self) -> Description;
}

pub trait Screen {
    type UiState: DrawWithin + Describe;

    fn entered(&mut self) {}
    fn left(&mut self) {}
    /// Changes whenever what the screen shows changed without an input: the screen is
    /// redrawn then. Screens that only change on input keep the default.
    fn version(&self) -> u64 {
        0
    }
    fn on_input(&mut self, input: Input);
    fn ui_state(&self) -> Self::UiState;
}

pub trait HostedScreen<D: DrawTarget<Color = BinaryColor>> {
    fn entered(&mut self);
    fn left(&mut self);
    fn version(&self) -> u64;
    fn on_input(&mut self, input: Input);
    fn draw(&self, target: &mut D, area: Rectangle) -> Description;
}

impl<D: DrawTarget<Color = BinaryColor>, S: Screen> HostedScreen<D> for S {
    fn entered(&mut self) {
        Screen::entered(self);
    }

    fn left(&mut self) {
        Screen::left(self);
    }

    fn version(&self) -> u64 {
        Screen::version(self)
    }

    fn on_input(&mut self, input: Input) {
        Screen::on_input(self, input);
    }

    fn draw(&self, target: &mut D, area: Rectangle) -> Description {
        let state = self.ui_state();
        state.draw_within(target, area);
        state.describe()
    }
}

pub struct InstalledApp<D> {
    pub service: Arc<dyn AppService>,
    pub screen: Box<dyn HostedScreen<D> + Send>,
}

pub type Install<D> = fn(&dyn Services) -> InstalledApp<D>;

pub struct Installable<D> {
    pub id: AppId,
    pub title: &'static str,
    pub install: Install<D>,
}
