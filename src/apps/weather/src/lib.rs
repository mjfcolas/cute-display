//! The weather app: today and the coming hours, or the week, where the device is.

mod ui;

use std::sync::Arc;

use ::domain::apps::{AppId, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use forecast::Weather;
use ::ui::{Installable, InstalledApp};

pub use crate::ui::screen::WeatherScreen;

pub const ID: AppId = AppId::new("weather");

/// This app, for an image to hold.
pub const fn app<D: DrawTarget<Color = BinaryColor>>() -> Installable<D> {
    Installable { id: ID, title: "Weather", install: install::<D> }
}

fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let weather = Weather::from_open_meteo(services);
    InstalledApp { service: Arc::new(weather.clone()), screen: Box::new(WeatherScreen::new(weather, services.clock())) }
}
