//! The settings as a few lines of text at the root of a storage:
//!
//! ```text
//! backlight = 10s
//! reading_lamp = 30%
//! ```
//!
//! A line per setting, so a damaged line loses that setting and not the others. Unknown
//! lines are ignored; a setting that is missing or unreadable takes its default.

use domain::settings::{BacklightDuration, ReadingLamp, SettingsRecord, SettingsStore};
use hal::storage::FileStorage;

pub const FILE_NAME: &str = "cute-display.conf";

const BACKLIGHT: &str = "backlight";
const READING_LAMP: &str = "reading_lamp";

pub struct SettingsFile<S> {
    storage: S,
}

impl<S: FileStorage> SettingsFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: FileStorage + Send> SettingsStore for SettingsFile<S> {
    fn load(&mut self) -> Option<SettingsRecord> {
        match self.storage.read(FILE_NAME) {
            Ok(contents) => contents.map(|bytes| decode(&String::from_utf8_lossy(&bytes))),
            Err(fault) => {
                log::warn!("settings: {fault}; starting on the defaults");
                None
            }
        }
    }

    fn save(&mut self, record: &SettingsRecord) {
        if let Err(fault) = self.storage.write(FILE_NAME, encode(record).as_bytes()) {
            log::warn!("settings: {fault}; the change holds until the power goes");
        }
    }
}

/// For a device with nowhere to keep its settings: they last until the power goes.
pub struct Unkept;

impl SettingsStore for Unkept {
    fn load(&mut self) -> Option<SettingsRecord> {
        None
    }

    fn save(&mut self, _: &SettingsRecord) {}
}

fn encode(record: &SettingsRecord) -> String {
    let backlight = match record.backlight {
        BacklightDuration::FiveSeconds => "5s",
        BacklightDuration::TenSeconds => "10s",
        BacklightDuration::ThirtySeconds => "30s",
        BacklightDuration::Always => "always",
    };
    let reading_lamp = match record.reading_lamp {
        ReadingLamp::Off => "off",
        ReadingLamp::TenPercent => "10%",
        ReadingLamp::ThirtyPercent => "30%",
        ReadingLamp::FiftyPercent => "50%",
        ReadingLamp::Full => "100%",
    };
    format!("{BACKLIGHT} = {backlight}\n{READING_LAMP} = {reading_lamp}\n")
}

fn decode(text: &str) -> SettingsRecord {
    let mut record = SettingsRecord::default();
    for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
        match (key.trim(), value.trim()) {
            (BACKLIGHT, "5s") => record.backlight = BacklightDuration::FiveSeconds,
            (BACKLIGHT, "10s") => record.backlight = BacklightDuration::TenSeconds,
            (BACKLIGHT, "30s") => record.backlight = BacklightDuration::ThirtySeconds,
            (BACKLIGHT, "always") => record.backlight = BacklightDuration::Always,
            (READING_LAMP, "off") => record.reading_lamp = ReadingLamp::Off,
            (READING_LAMP, "10%") => record.reading_lamp = ReadingLamp::TenPercent,
            (READING_LAMP, "30%") => record.reading_lamp = ReadingLamp::ThirtyPercent,
            (READING_LAMP, "50%") => record.reading_lamp = ReadingLamp::FiftyPercent,
            (READING_LAMP, "100%") => record.reading_lamp = ReadingLamp::Full,
            _ => {}
        }
    }
    record
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use hal::Fault;

    use super::*;

    #[derive(Clone, Default)]
    struct FakeStorage(Arc<Mutex<HashMap<String, Vec<u8>>>>);

    impl FileStorage for FakeStorage {
        fn root_entries(&self) -> Result<Vec<String>, Fault> {
            Ok(self.0.lock().unwrap().keys().cloned().collect())
        }
        fn capacity_bytes(&self) -> Result<u64, Fault> {
            Ok(0)
        }
        fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
            Ok(self.0.lock().unwrap().get(path).cloned())
        }
        fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
            self.0.lock().unwrap().insert(path.into(), contents.to_vec());
            Ok(())
        }
    }

    #[test]
    fn every_record_comes_back_as_it_was_saved() {
        let storage = FakeStorage::default();
        let mut file = SettingsFile::new(storage.clone());
        for backlight in [
            BacklightDuration::FiveSeconds,
            BacklightDuration::TenSeconds,
            BacklightDuration::ThirtySeconds,
            BacklightDuration::Always,
        ] {
            for reading_lamp in [
                ReadingLamp::Off,
                ReadingLamp::TenPercent,
                ReadingLamp::ThirtyPercent,
                ReadingLamp::FiftyPercent,
                ReadingLamp::Full,
            ] {
                let record = SettingsRecord { backlight, reading_lamp };
                file.save(&record);
                assert_eq!(file.load(), Some(record));
            }
        }
    }

    #[test]
    fn no_file_is_no_record() {
        assert_eq!(SettingsFile::new(FakeStorage::default()).load(), None);
    }

    #[test]
    fn a_damaged_line_loses_only_its_own_setting() {
        let record = decode("backlight = forever\n# a comment\nreading_lamp = 50%\nvolume = 11\n");
        assert_eq!(record, SettingsRecord { backlight: BacklightDuration::default(), reading_lamp: ReadingLamp::FiftyPercent });
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        let text = encode(&SettingsRecord { backlight: BacklightDuration::ThirtySeconds, reading_lamp: ReadingLamp::ThirtyPercent });
        assert_eq!(text, "backlight = 30s\nreading_lamp = 30%\n");
    }
}
