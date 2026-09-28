//! The airports around the radar's place, one a line: code, latitude, longitude, then a
//! name for whoever reads the file.
//!
//! ```text
//! LFPG 49.0097 2.5479 Paris Charles de Gaulle Airport
//! LFPO 48.7233 2.3794 Paris-Orly Airport
//! ```
//!
//! The installer (`just radar-airports`) writes it from OurAirports. A line that does
//! not read is skipped, and so is a `#` comment.
//!
//! Which airports are named on the scope is the radar's own setting, since this file is
//! written again whenever the place changes:
//!
//! ```text
//! # cute-display/radar.conf
//! airport_labels = LFPG, LFPO, LFPB
//! ```

use domain::place::GeoPoint;
use domain::radar::{Airport, AirportSource};
use hal::storage::FileStorage;

use crate::conf_text::ConfText;
use crate::place_file::RADAR_FILE;

pub const FILE_NAME: &str = "cute-display/airports.conf";
const LABELS: &str = "airport_labels";

pub struct AirportsFile<S> {
    storage: S,
}

impl<S: FileStorage> AirportsFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: FileStorage + Send> AirportSource for AirportsFile<S> {
    fn airports(&mut self) -> Vec<Airport> {
        let mut airports: Vec<Airport> = match self.storage.read(FILE_NAME) {
            Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).lines().filter_map(airport).collect(),
            Ok(None) => Vec::new(),
            Err(fault) => {
                log::warn!("{FILE_NAME}: {fault}");
                Vec::new()
            }
        };
        let labelled = self.labelled();
        for airport in &mut airports {
            airport.labelled = labelled.iter().any(|code| code.eq_ignore_ascii_case(&airport.code));
        }
        airports
    }
}

impl<S: FileStorage> AirportsFile<S> {
    /// The codes listed in `airport_labels`, separated by commas or spaces.
    fn labelled(&self) -> Vec<String> {
        let Ok(Some(conf)) = ConfText::read(&self.storage, RADAR_FILE) else {
            return Vec::new();
        };
        let listed = conf.get(LABELS).unwrap_or_default();
        listed.split([',', ' ']).filter(|code| !code.is_empty()).map(str::to_owned).collect()
    }
}

fn airport(line: &str) -> Option<Airport> {
    let mut words = line.split_whitespace();
    let code = words.next().filter(|c| !c.starts_with('#'))?;
    let latitude = words.next()?.parse::<f64>().ok().filter(|v| v.abs() <= 90.0)?;
    let longitude = words.next()?.parse::<f64>().ok().filter(|v| v.abs() <= 180.0)?;
    Some(Airport { code: code.to_owned(), point: GeoPoint { latitude, longitude }, labelled: false })
}

/// For a device with nowhere to read airports from.
pub struct NoAirports;

impl AirportSource for NoAirports {
    fn airports(&mut self) -> Vec<Airport> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    #[test]
    fn airports_are_read_one_a_line_and_bad_lines_skipped() {
        let text = "# around Notre-Dame\nLFPG 49.0097 2.5479 Paris Charles de Gaulle Airport\nLFPO 48.7233 2.3794\nnonsense\nLFXX north 2\n";
        let airports = AirportsFile::new(MemoryStorage::with(FILE_NAME, text)).airports();
        let codes: Vec<&str> = airports.iter().map(|a| a.code.as_str()).collect();
        assert_eq!(codes, ["LFPG", "LFPO"]);
        assert_eq!(airports[0].point, GeoPoint { latitude: 49.0097, longitude: 2.5479 });
    }

    #[test]
    fn the_radar_names_the_airports_it_lists() {
        let storage = MemoryStorage::with(FILE_NAME, "LFPG 49.0097 2.5479\nLFPO 48.7233 2.3794\nLFPB 48.9694 2.4414\n");
        storage.write(RADAR_FILE, b"place = Notre-Dame\nairport_labels = LFPG,lfpo\n").unwrap();
        let labelled: Vec<(String, bool)> = AirportsFile::new(storage).airports().into_iter().map(|a| (a.code, a.labelled)).collect();
        assert_eq!(labelled, [("LFPG".into(), true), ("LFPO".into(), true), ("LFPB".into(), false)]);
    }

    #[test]
    fn no_file_is_no_airports() {
        assert!(AirportsFile::new(MemoryStorage::default()).airports().is_empty());
    }
}
