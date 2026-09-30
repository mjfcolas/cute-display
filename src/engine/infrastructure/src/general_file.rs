use conf_text::ConfText;
use domain::clock::TimeZoneSource;
use domain::fetch::Unavailable;
use domain::place::{GeoPoint, Place, PlaceSource};
use domain::time_zone::TimeZone;
use hal::storage::FileStorage;

use crate::conf_file;

pub const GENERAL_FILE: &str = "cute-display/general.conf";
const TIME_ZONE: &str = "time_zone";
const APPS: &str = "apps";

pub struct GeneralFile<S> {
    storage: S,
    /// The time zone last warned about, so that a lasting mistake is logged once.
    unreadable: Option<String>,
}

impl<S: FileStorage> GeneralFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage, unreadable: None }
    }

    pub fn apps(&self) -> Option<Vec<String>> {
        let conf = self.conf().inspect_err(|unavailable| log::warn!("apps: {unavailable}")).ok()??;
        let listed = conf.get(APPS)?;
        Some(listed.split([',', ' ']).filter(|name| !name.is_empty()).map(str::to_owned).collect())
    }

    fn conf(&self) -> Result<Option<ConfText>, Unavailable> {
        conf_file::read(&self.storage, GENERAL_FILE).map_err(|fault| Unavailable(format!("{GENERAL_FILE}: {fault}")))
    }
}

impl<S: FileStorage + Send> TimeZoneSource for GeneralFile<S> {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        let conf = self.conf().inspect_err(|unavailable| log::warn!("clock: {unavailable}"))?;
        let Some(text) = conf.as_ref().and_then(|conf| conf.get(TIME_ZONE)) else {
            return Ok(None);
        };
        let zone = TimeZone::parse(text);
        if zone.is_none() && self.unreadable.as_deref() != Some(text) {
            log::warn!("clock: {GENERAL_FILE}: cannot read the time zone {text:?}");
            self.unreadable = Some(text.to_owned());
        }
        zone.map(Some).ok_or_else(|| Unavailable(format!("{GENERAL_FILE}: cannot read the time zone {text:?}")))
    }
}

impl<S: FileStorage + Send> PlaceSource for GeneralFile<S> {
    fn place(&mut self) -> Option<Place> {
        let conf = match self.conf() {
            Ok(conf) => conf?,
            Err(unavailable) => {
                log::warn!("{unavailable}");
                return None;
            }
        };
        let coordinate = |key: &str, limit: f64| conf.get(key).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.abs() <= limit);
        let (Some(latitude), Some(longitude)) = (coordinate("latitude", 90.0), coordinate("longitude", 180.0)) else {
            log::warn!("{GENERAL_FILE} needs a latitude and a longitude, in decimal degrees");
            return None;
        };
        let name = conf.get("place").filter(|p| !p.is_empty()).unwrap_or("Here").to_owned();
        Some(Place { name, point: GeoPoint { latitude, longitude } })
    }
}

pub struct NoGeneralFile;

impl TimeZoneSource for NoGeneralFile {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        Ok(None)
    }
}

impl PlaceSource for NoGeneralFile {
    fn place(&mut self) -> Option<Place> {
        None
    }
}

#[cfg(test)]
mod tests {
    use hal_testing::storage::FakeFileStorage;

    use super::*;

    fn general(text: &str) -> GeneralFile<FakeFileStorage> {
        GeneralFile::new(FakeFileStorage::with(GENERAL_FILE, text))
    }

    #[test]
    fn reads_the_time_zone_of_the_file() {
        assert_eq!(general("time_zone = UTC0\n").time_zone(), Ok(Some(TimeZone::UTC)));
    }

    #[test]
    fn no_file_or_no_line_is_none_chosen_and_nonsense_cannot_be_read() {
        assert_eq!(GeneralFile::new(FakeFileStorage::default()).time_zone(), Ok(None));
        assert_eq!(general("# nothing\n").time_zone(), Ok(None));
        assert!(general("time_zone = Europe/Paris\n").time_zone().is_err());
    }

    #[test]
    fn a_place_is_read_with_its_coordinates() {
        let read = general("place = Montréal\nlatitude = 45.50\nlongitude = -73.57\n").place();
        assert_eq!(read, Some(Place { name: "Montréal".into(), point: GeoPoint { latitude: 45.5, longitude: -73.57 } }));
    }

    #[test]
    fn coordinates_are_required_and_must_be_on_earth() {
        assert_eq!(general("place = Paris\n").place(), None);
        assert_eq!(general("latitude = 95\nlongitude = 2\n").place(), None);
        assert_eq!(general("latitude = north\nlongitude = 2\n").place(), None);
        assert_eq!(GeneralFile::new(FakeFileStorage::default()).place(), None);
    }

    #[test]
    fn a_place_without_a_name_is_still_a_place() {
        assert_eq!(general("latitude = 1\nlongitude = 2\n").place().map(|p| p.name), Some("Here".into()));
    }

    #[test]
    fn the_place_and_the_time_zone_share_the_file() {
        let mut file = general("latitude = 1\nlongitude = 2\ntime_zone = UTC0\n");
        assert!(file.place().is_some());
        assert_eq!(file.time_zone(), Ok(Some(TimeZone::UTC)));
    }

    #[test]
    fn the_apps_are_listed_by_name_and_none_listed_is_not_saying() {
        assert_eq!(general("apps = alarm, radar weather\n").apps(), Some(vec!["alarm".into(), "radar".into(), "weather".into()]));
        assert_eq!(general("apps =\n").apps(), Some(vec![]));
        assert_eq!(general("time_zone = UTC0\n").apps(), None);
        assert_eq!(GeneralFile::new(FakeFileStorage::default()).apps(), None);
    }
}
