//! A place, as a person writes it in a conf file:
//!
//! ```text
//! place = Notre-Dame
//! latitude = 48.8530
//! longitude = 2.3499
//! ```

use domain::place::{GeoPoint, Place, PlaceSource};
use hal::storage::FileStorage;

use crate::conf_text::ConfText;

pub const WEATHER_FILE: &str = "cute-display/weather.conf";
pub const RADAR_FILE: &str = "cute-display/radar.conf";

pub struct PlaceFile<S> {
    storage: S,
    path: &'static str,
}

impl<S: FileStorage> PlaceFile<S> {
    pub fn new(storage: S, path: &'static str) -> Self {
        Self { storage, path }
    }
}

impl<S: FileStorage + Send> PlaceSource for PlaceFile<S> {
    fn place(&mut self) -> Option<Place> {
        let conf = match ConfText::read(&self.storage, self.path) {
            Ok(conf) => conf?,
            Err(fault) => {
                log::warn!("{}: {fault}", self.path);
                return None;
            }
        };
        let coordinate = |key: &str, limit: f64| conf.get(key).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.abs() <= limit);
        let (Some(latitude), Some(longitude)) = (coordinate("latitude", 90.0), coordinate("longitude", 180.0)) else {
            log::warn!("{} needs a latitude and a longitude, in decimal degrees", self.path);
            return None;
        };
        let name = conf.get("place").filter(|p| !p.is_empty()).unwrap_or("Here").to_owned();
        Some(Place { name, point: GeoPoint { latitude, longitude } })
    }
}

/// For a device with nowhere to read a place from.
pub struct NoPlace;

impl PlaceSource for NoPlace {
    fn place(&mut self) -> Option<Place> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    fn place(text: &str) -> Option<Place> {
        PlaceFile::new(MemoryStorage::with(RADAR_FILE, text), RADAR_FILE).place()
    }

    #[test]
    fn a_place_is_read_with_its_coordinates() {
        let read = place("place = Montréal\nlatitude = 45.50\nlongitude = -73.57\n");
        assert_eq!(read, Some(Place { name: "Montréal".into(), point: GeoPoint { latitude: 45.5, longitude: -73.57 } }));
    }

    #[test]
    fn coordinates_are_required_and_must_be_on_earth() {
        assert_eq!(place("place = Paris\n"), None);
        assert_eq!(place("latitude = 95\nlongitude = 2\n"), None);
        assert_eq!(place("latitude = north\nlongitude = 2\n"), None);
    }

    #[test]
    fn a_place_without_a_name_is_still_a_place() {
        assert_eq!(place("latitude = 1\nlongitude = 2\n").map(|p| p.name), Some("Here".into()));
    }

    #[test]
    fn each_file_is_its_own_place() {
        let storage = MemoryStorage::with(WEATHER_FILE, "latitude = 1\nlongitude = 2\n");
        assert!(PlaceFile::new(storage.clone(), WEATHER_FILE).place().is_some());
        assert!(PlaceFile::new(storage, RADAR_FILE).place().is_none());
    }
}
