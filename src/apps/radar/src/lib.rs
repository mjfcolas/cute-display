//! The radar app: the aircraft around the device, from adsb.fi, and the airports in its
//! `airports.conf`.

mod domain;
mod infrastructure;
mod ui;

use std::sync::Arc;

use ::domain::apps::{AppId, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use ::ui::{Installable, InstalledApp};

pub use crate::domain::radar::{AirTrafficSource, Aircraft, Airport, AirportSource, Altitude, Radar, RadarReport, Range};
pub use crate::ui::screen::RadarScreen;

use crate::infrastructure::adsb_fi::AdsbFi;
use crate::infrastructure::airports_file::AirportsFile;

pub const ID: AppId = AppId::new("radar");

/// This app, for an image to hold.
pub const fn app<D: DrawTarget<Color = BinaryColor>>() -> Installable<D> {
    Installable { id: ID, title: "Radar", install: install::<D> }
}

fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let radar = Radar::new(
        services.place(),
        Box::new(AdsbFi::new(services.internet())),
        Box::new(AirportsFile::new(services.files())),
        services.foreground(),
    );
    InstalledApp { service: Arc::new(radar.clone()), screen: Box::new(RadarScreen::new(radar)) }
}
