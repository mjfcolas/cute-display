//! Where the weather is for, as a person writes it:
//!
//! ```text
//! place = Paris
//! latitude = 48.85
//! longitude = 2.35
//! ```

use domain::weather::{Location, LocationSource};
use hal::storage::FileStorage;

use crate::conf_text::ConfText;

pub const FILE_NAME: &str = "cute-display/weather.conf";

pub struct LocationFile<S> {
    storage: S,
}

impl<S: FileStorage> LocationFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: FileStorage + Send> LocationSource for LocationFile<S> {
    fn location(&mut self) -> Option<Location> {
        let conf = match ConfText::read(&self.storage, FILE_NAME) {
            Ok(conf) => conf?,
            Err(fault) => {
                log::warn!("weather: {fault}");
                return None;
            }
        };
        let coordinate = |key: &str, limit: f64| {
            conf.get(key).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.abs() <= limit)
        };
        let (Some(latitude), Some(longitude)) = (coordinate("latitude", 90.0), coordinate("longitude", 180.0)) else {
            log::warn!("weather: {FILE_NAME} needs a latitude and a longitude, in decimal degrees");
            return None;
        };
        let place = conf.get("place").filter(|p| !p.is_empty()).unwrap_or("Here").to_owned();
        Some(Location { place, latitude, longitude })
    }
}

/// For a device with nowhere to read a place from.
pub struct NoPlace;

impl LocationSource for NoPlace {
    fn location(&mut self) -> Option<Location> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    fn location(text: &str) -> Option<Location> {
        LocationFile::new(MemoryStorage::with(FILE_NAME, text)).location()
    }

    #[test]
    fn a_place_is_read_with_its_coordinates() {
        let read = location("place = Montréal\nlatitude = 45.50\nlongitude = -73.57\n");
        assert_eq!(read, Some(Location { place: "Montréal".into(), latitude: 45.5, longitude: -73.57 }));
    }

    #[test]
    fn coordinates_are_required_and_must_be_on_earth() {
        assert_eq!(location("place = Paris\n"), None);
        assert_eq!(location("latitude = 95\nlongitude = 2\n"), None);
        assert_eq!(location("latitude = north\nlongitude = 2\n"), None);
    }

    #[test]
    fn a_place_without_a_name_is_still_a_place() {
        assert_eq!(location("latitude = 1\nlongitude = 2\n").map(|l| l.place), Some("Here".into()));
    }

    #[test]
    fn no_file_is_no_place() {
        assert_eq!(LocationFile::new(MemoryStorage::default()).location(), None);
    }
}
