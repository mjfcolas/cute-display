//! The weather app: today and the coming hours, or the week, at the place in
//! `cute-display/weather.conf`.

mod ui;

use std::sync::Arc;

use ::domain::apps::{AppId, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use forecast::Weather;
use ::ui::InstalledApp;

pub use crate::ui::screen::WeatherScreen;

pub const ID: AppId = AppId::new("weather");

pub fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let weather = Weather::from_open_meteo(services);
    InstalledApp { service: Arc::new(weather.clone()), screen: Box::new(WeatherScreen::new(weather, services.clock())) }
}
