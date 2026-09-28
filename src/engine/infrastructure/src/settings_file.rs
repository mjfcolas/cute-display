//! The settings as a few lines of text, in the device's own directory of a storage:
//!
//! ```text
//! backlight = 10s
//! reading_lamp = 30%
//! ```
//!
//! A setting that is missing or unreadable takes its default.

use conf_text::ConfText;
use domain::settings::{BacklightDuration, ReadingLamp, SettingsRecord, SettingsStore};
use hal::storage::FileStorage;


use crate::conf_file;

pub const FILE_NAME: &str = "cute-display/settings.conf";

const BACKLIGHT: &str = "backlight";
const READING_LAMP: &str = "reading_lamp";

const BACKLIGHT_VALUES: [(BacklightDuration, &str); 4] = [
    (BacklightDuration::FiveSeconds, "5s"),
    (BacklightDuration::TenSeconds, "10s"),
    (BacklightDuration::ThirtySeconds, "30s"),
    (BacklightDuration::Always, "always"),
];

const READING_LAMP_VALUES: [(ReadingLamp, &str); 5] = [
    (ReadingLamp::Off, "off"),
    (ReadingLamp::TenPercent, "10%"),
    (ReadingLamp::ThirtyPercent, "30%"),
    (ReadingLamp::FiftyPercent, "50%"),
    (ReadingLamp::Full, "100%"),
];

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
        match conf_file::read(&self.storage, FILE_NAME) {
            Ok(conf) => conf.map(|conf| decode(&conf)),
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

fn name_of<T: PartialEq + Copy>(values: &[(T, &'static str)], value: T) -> &'static str {
    values.iter().find(|(v, _)| *v == value).map_or("", |(_, name)| name)
}

fn value_of<T: Copy>(values: &[(T, &str)], name: Option<&str>) -> Option<T> {
    values.iter().find(|(_, n)| Some(*n) == name).map(|(value, _)| *value)
}

fn encode(record: &SettingsRecord) -> String {
    ConfText::render(&[
        (BACKLIGHT, name_of(&BACKLIGHT_VALUES, record.backlight)),
        (READING_LAMP, name_of(&READING_LAMP_VALUES, record.reading_lamp)),
    ])
}

fn decode(conf: &ConfText) -> SettingsRecord {
    let defaults = SettingsRecord::default();
    SettingsRecord {
        backlight: value_of(&BACKLIGHT_VALUES, conf.get(BACKLIGHT)).unwrap_or(defaults.backlight),
        reading_lamp: value_of(&READING_LAMP_VALUES, conf.get(READING_LAMP)).unwrap_or(defaults.reading_lamp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    #[test]
    fn every_record_comes_back_as_it_was_saved() {
        let mut file = SettingsFile::new(MemoryStorage::default());
        for (backlight, _) in BACKLIGHT_VALUES {
            for (reading_lamp, _) in READING_LAMP_VALUES {
                let record = SettingsRecord { backlight, reading_lamp };
                file.save(&record);
                assert_eq!(file.load(), Some(record));
            }
        }
    }

    #[test]
    fn the_file_lives_in_the_devices_own_directory() {
        let storage = MemoryStorage::default();
        SettingsFile::new(storage.clone()).save(&SettingsRecord::default());
        assert!(storage.contains("cute-display/settings.conf"));
    }

    #[test]
    fn no_file_is_no_record() {
        assert_eq!(SettingsFile::new(MemoryStorage::default()).load(), None);
    }

    #[test]
    fn a_damaged_line_loses_only_its_own_setting() {
        let storage = MemoryStorage::with(FILE_NAME, "backlight = forever\n# a comment\nreading_lamp = 50%\nvolume = 11\n");
        let record = SettingsFile::new(storage).load();
        assert_eq!(record, Some(SettingsRecord { backlight: BacklightDuration::default(), reading_lamp: ReadingLamp::FiftyPercent }));
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        let text = encode(&SettingsRecord { backlight: BacklightDuration::ThirtySeconds, reading_lamp: ReadingLamp::ThirtyPercent });
        assert_eq!(text, "backlight = 30s\nreading_lamp = 30%\n");
    }
}
