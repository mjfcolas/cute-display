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
//! # cute-display/apps/radar/radar.conf
//! airport_labels = LFPG, LFPO, LFPB
//! ```

use conf_text::ConfText;
use domain::files::Files;
use domain::place::GeoPoint;

use crate::domain::radar::{Airport, AirportSource};

const AIRPORTS_FILE: &str = "airports.conf";
const RADAR_FILE: &str = "radar.conf";
const LABELS: &str = "airport_labels";

pub struct AirportsFile {
    files: Box<dyn Files>,
}

impl AirportsFile {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl AirportSource for AirportsFile {
    fn airports(&mut self) -> Vec<Airport> {
        let mut airports: Vec<Airport> = match self.files.read(AIRPORTS_FILE) {
            Ok(Some(text)) => text.lines().filter_map(airport).collect(),
            Ok(None) => Vec::new(),
            Err(unavailable) => {
                log::warn!("{unavailable}");
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

impl AirportsFile {
    /// The codes listed in `airport_labels`, separated by commas or spaces.
    fn labelled(&self) -> Vec<String> {
        let Ok(Some(text)) = self.files.read(RADAR_FILE) else {
            return Vec::new();
        };
        let conf = ConfText::parse(&text);
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

    fn with(name: &str, text: &str) -> MemoryFiles {
        let files = MemoryFiles::default();
        files.write(name, text).unwrap();
        files
    }

    #[test]
    fn airports_are_read_one_a_line_and_bad_lines_skipped() {
        let text = "# around Notre-Dame\nLFPG 49.0097 2.5479 Paris Charles de Gaulle Airport\nLFPO 48.7233 2.3794\nnonsense\nLFXX north 2\n";
        let airports = AirportsFile::new(Box::new(with(AIRPORTS_FILE, text))).airports();
        let codes: Vec<&str> = airports.iter().map(|a| a.code.as_str()).collect();
        assert_eq!(codes, ["LFPG", "LFPO"]);
        assert_eq!(airports[0].point, GeoPoint { latitude: 49.0097, longitude: 2.5479 });
    }

    #[test]
    fn the_radar_names_the_airports_it_lists() {
        let files = with(AIRPORTS_FILE, "LFPG 49.0097 2.5479\nLFPO 48.7233 2.3794\nLFPB 48.9694 2.4414\n");
        files.write(RADAR_FILE, "airport_labels = LFPG,lfpo\n").unwrap();
        let labelled: Vec<(String, bool)> = AirportsFile::new(Box::new(files)).airports().into_iter().map(|a| (a.code, a.labelled)).collect();
        assert_eq!(labelled, [("LFPG".into(), true), ("LFPO".into(), true), ("LFPB".into(), false)]);
    }

    #[test]
    fn no_file_is_no_airports() {
        assert!(AirportsFile::new(Box::new(MemoryFiles::default())).airports().is_empty());
    }
}
