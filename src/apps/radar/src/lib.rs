//! The radar app: the aircraft around the place in `cute-display/radar.conf`, from
//! adsb.fi, and the airports in `cute-display/airports.conf`.

mod domain;
mod infrastructure;
mod ui;

use std::sync::Arc;

use ::domain::apps::{AppId, Services};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use place::PlaceFile;
use ::ui::InstalledApp;

pub use crate::domain::radar::{AirTrafficSource, Aircraft, Airport, AirportSource, Altitude, Radar, RadarReport, Range};
pub use crate::ui::screen::RadarScreen;

use crate::infrastructure::adsb_fi::AdsbFi;
use crate::infrastructure::airports_file::{self, AirportsFile};

pub const ID: AppId = AppId::new("radar");

/// Where the radar looks from, and which airports it names.
pub const PLACE_FILE: &str = "radar.conf";
const FILES: &[&str] = &[PLACE_FILE, airports_file::FILE_NAME];

pub fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let radar = Radar::new(
        Box::new(PlaceFile::new(services.files(FILES), PLACE_FILE)),
        Box::new(AdsbFi::new(services.internet())),
        Box::new(AirportsFile::new(services.files(FILES))),
        services.foreground(),
    );
    InstalledApp { service: Arc::new(radar.clone()), screen: Box::new(RadarScreen::new(radar)) }
}
