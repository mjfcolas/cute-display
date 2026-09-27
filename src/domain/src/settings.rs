//! What the device remembers across a power cut.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::lighting::Level;
use crate::shared::Shared;

/// How long the light over the screen stays on after the controls were last touched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BacklightDuration {
    FiveSeconds,
    #[default]
    TenSeconds,
    ThirtySeconds,
    Always,
}

impl BacklightDuration {
    /// Every choice, in the order [`BacklightDuration::next`] steps through them.
    pub const ALL: [Self; 4] = [Self::FiveSeconds, Self::TenSeconds, Self::ThirtySeconds, Self::Always];

    /// `None` for [`BacklightDuration::Always`].
    pub fn duration(self) -> Option<Duration> {
        match self {
            Self::FiveSeconds => Some(Duration::from_secs(5)),
            Self::TenSeconds => Some(Duration::from_secs(10)),
            Self::ThirtySeconds => Some(Duration::from_secs(30)),
            Self::Always => None,
        }
    }

    /// The next choice, back to the first after the last.
    pub fn next(self) -> Self {
        following(&Self::ALL, self)
    }
}

/// How bright the reading lamp shines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadingLamp {
    #[default]
    Off,
    TenPercent,
    ThirtyPercent,
    FiftyPercent,
    Full,
}

impl ReadingLamp {
    /// Every choice, in the order [`ReadingLamp::next`] steps through them.
    pub const ALL: [Self; 5] = [Self::Off, Self::TenPercent, Self::ThirtyPercent, Self::FiftyPercent, Self::Full];

    pub fn level(self) -> Level {
        Level::percent(match self {
            Self::Off => 0,
            Self::TenPercent => 10,
            Self::ThirtyPercent => 30,
            Self::FiftyPercent => 50,
            Self::Full => 100,
        })
    }

    /// The next choice, back to off after full.
    pub fn next(self) -> Self {
        following(&Self::ALL, self)
    }
}

/// The choice after `choice` in `all`, back to the first after the last.
fn following<T: Copy + PartialEq>(all: &[T], choice: T) -> T {
    let after = all.iter().skip_while(|&&c| c != choice).nth(1);
    after.or(all.first()).copied().unwrap_or(choice)
}

/// Every setting, as it is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsRecord {
    pub backlight: BacklightDuration,
    pub reading_lamp: ReadingLamp,
}

/// Where the settings are kept. A store that cannot be read or written leaves the device
/// running on the settings it has in memory.
pub trait SettingsStore: Send {
    /// `None` when nothing has been kept yet.
    fn load(&mut self) -> Option<SettingsRecord>;
    fn save(&mut self, record: &SettingsRecord);
}

/// The settings, kept by a store on every change. Every clone is the same.
#[derive(Clone)]
pub struct Settings {
    record: Shared<SettingsRecord>,
    store: Arc<Mutex<Box<dyn SettingsStore>>>,
}

impl Settings {
    /// What the store kept, or the defaults.
    pub fn load(mut store: Box<dyn SettingsStore>) -> Self {
        let record = store.load().unwrap_or_default();
        Self { record: Shared::new(record), store: Arc::new(Mutex::new(store)) }
    }

    pub fn backlight(&self) -> BacklightDuration {
        self.record.get().backlight
    }

    pub fn reading_lamp(&self) -> ReadingLamp {
        self.record.get().reading_lamp
    }

    pub fn choose_next_backlight(&self) {
        self.change(|record| record.backlight = record.backlight.next());
    }

    pub fn choose_next_reading_lamp(&self) {
        self.change(|record| record.reading_lamp = record.reading_lamp.next());
    }

    fn change(&self, change: impl FnOnce(&mut SettingsRecord)) {
        self.record.update(change);
        let record = self.record.get();
        match self.store.lock() {
            Ok(mut store) => store.save(&record),
            Err(poisoned) => poisoned.into_inner().save(&record),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct FakeStore {
        kept: Arc<Mutex<Option<SettingsRecord>>>,
    }

    impl SettingsStore for FakeStore {
        fn load(&mut self) -> Option<SettingsRecord> {
            *self.kept.lock().unwrap()
        }
        fn save(&mut self, record: &SettingsRecord) {
            *self.kept.lock().unwrap() = Some(*record);
        }
    }

    #[test]
    fn a_device_that_kept_nothing_starts_on_the_defaults() {
        let settings = Settings::load(Box::new(FakeStore::default()));
        assert_eq!(settings.backlight(), BacklightDuration::TenSeconds);
        assert_eq!(settings.reading_lamp(), ReadingLamp::Off);
    }

    #[test]
    fn every_change_is_kept_and_comes_back_on_the_next_load() {
        let store = FakeStore::default();
        let settings = Settings::load(Box::new(store.clone()));
        settings.choose_next_backlight();
        settings.choose_next_reading_lamp();

        let reloaded = Settings::load(Box::new(store));
        assert_eq!(reloaded.backlight(), BacklightDuration::ThirtySeconds);
        assert_eq!(reloaded.reading_lamp(), ReadingLamp::TenPercent);
    }

    #[test]
    fn the_backlight_choices_come_round_again() {
        let mut duration = BacklightDuration::FiveSeconds;
        let mut seen = vec![duration];
        for _ in 0..4 {
            duration = duration.next();
            seen.push(duration);
        }
        assert_eq!(
            seen,
            [
                BacklightDuration::FiveSeconds,
                BacklightDuration::TenSeconds,
                BacklightDuration::ThirtySeconds,
                BacklightDuration::Always,
                BacklightDuration::FiveSeconds,
            ]
        );
    }

    #[test]
    fn the_reading_lamp_goes_through_its_levels_and_back_to_off() {
        let mut lamp = ReadingLamp::Off;
        let mut percents = vec![];
        for _ in 0..6 {
            percents.push(lamp.level().as_percent());
            lamp = lamp.next();
        }
        assert_eq!(percents, [0, 10, 30, 50, 100, 0]);
    }
}
