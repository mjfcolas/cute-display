//! The time zone, as a person writes it in a conf file, in POSIX `TZ` form:
//!
//! ```text
//! time_zone = CET-1CEST,M3.5.0,M10.5.0/3
//! ```

use domain::clock::TimeZoneSource;
use domain::fetch::Unavailable;
use domain::time_zone::TimeZone;
use hal::storage::FileStorage;

use crate::conf_text::ConfText;

pub const CLOCK_FILE: &str = "cute-display/clock.conf";
const TIME_ZONE: &str = "time_zone";

pub struct TimeZoneFile<S> {
    storage: S,
    /// The text last warned about, so that a lasting mistake is logged once.
    unreadable: Option<String>,
}

impl<S: FileStorage> TimeZoneFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage, unreadable: None }
    }
}

impl<S: FileStorage + Send> TimeZoneSource for TimeZoneFile<S> {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        let conf = ConfText::read(&self.storage, CLOCK_FILE).map_err(|fault| {
            log::warn!("clock: {fault}");
            Unavailable(fault.to_string())
        })?;
        let Some(text) = conf.as_ref().and_then(|conf| conf.get(TIME_ZONE)) else {
            return Ok(None);
        };
        let zone = TimeZone::parse(text);
        if zone.is_none() && self.unreadable.as_deref() != Some(text) {
            log::warn!("clock: {CLOCK_FILE}: cannot read the time zone {text:?}");
            self.unreadable = Some(text.to_owned());
        }
        zone.map(Some).ok_or_else(|| Unavailable(format!("{CLOCK_FILE}: cannot read the time zone {text:?}")))
    }
}

/// For a device with nowhere to read a time zone from: it keeps the default.
pub struct NoTimeZone;

impl TimeZoneSource for NoTimeZone {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    #[test]
    fn reads_the_time_zone_of_the_file() {
        let storage = MemoryStorage::with(CLOCK_FILE, "time_zone = UTC0\n");
        assert_eq!(TimeZoneFile::new(storage).time_zone(), Ok(Some(TimeZone::UTC)));
    }

    #[test]
    fn no_file_or_no_line_is_none_chosen_and_nonsense_cannot_be_read() {
        assert_eq!(TimeZoneFile::new(MemoryStorage::default()).time_zone(), Ok(None));
        assert_eq!(TimeZoneFile::new(MemoryStorage::with(CLOCK_FILE, "# nothing\n")).time_zone(), Ok(None));
        assert!(TimeZoneFile::new(MemoryStorage::with(CLOCK_FILE, "time_zone = Europe/Paris\n")).time_zone().is_err());
    }
}
