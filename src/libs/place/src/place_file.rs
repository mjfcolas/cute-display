//! A place, as a person writes it in a conf file:
//!
//! ```text
//! place = Notre-Dame
//! latitude = 48.8530
//! longitude = 2.3499
//! ```

use conf_text::ConfText;
use domain::files::Files;

use crate::{GeoPoint, Place, PlaceSource};

pub struct PlaceFile {
    files: Box<dyn Files>,
    name: &'static str,
}

impl PlaceFile {
    pub fn new(files: Box<dyn Files>, name: &'static str) -> Self {
        Self { files, name }
    }
}

impl PlaceSource for PlaceFile {
    fn place(&mut self) -> Option<Place> {
        let conf = match self.files.read(self.name) {
            Ok(text) => ConfText::parse(&text?),
            Err(unavailable) => {
                log::warn!("{unavailable}");
                return None;
            }
        };
        let coordinate = |key: &str, limit: f64| conf.get(key).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.abs() <= limit);
        let (Some(latitude), Some(longitude)) = (coordinate("latitude", 90.0), coordinate("longitude", 180.0)) else {
            log::warn!("{} needs a latitude and a longitude, in decimal degrees", self.name);
            return None;
        };
        let name = conf.get("place").filter(|p| !p.is_empty()).unwrap_or("Here").to_owned();
        Some(Place { name, point: GeoPoint { latitude, longitude } })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use domain::fetch::Unavailable;

    use super::*;

    /// Files in memory; every clone sees the same ones.
    #[derive(Clone, Default)]
    struct MemoryFiles(Arc<Mutex<BTreeMap<String, String>>>);

    impl Files for MemoryFiles {
        fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
            Ok(self.0.lock().unwrap().get(name).cloned())
        }
        fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
            self.0.lock().unwrap().insert(name.into(), text.into());
            Ok(())
        }
    }

    const WEATHER_FILE: &str = "weather.conf";
    const RADAR_FILE: &str = "radar.conf";

    fn with(name: &str, text: &str) -> MemoryFiles {
        let files = MemoryFiles::default();
        files.write(name, text).unwrap();
        files
    }

    fn place(text: &str) -> Option<Place> {
        PlaceFile::new(Box::new(with(RADAR_FILE, text)), RADAR_FILE).place()
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
        let files = with(WEATHER_FILE, "latitude = 1\nlongitude = 2\n");
        assert!(PlaceFile::new(Box::new(files.clone()), WEATHER_FILE).place().is_some());
        assert!(PlaceFile::new(Box::new(files), RADAR_FILE).place().is_none());
    }
}
