use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use crate::settings::{BacklightDuration, Settings};

/// Enough to read the screen in the dark, not enough to light the room.
const BACKLIGHT_LEVEL: Level = Level::percent(20);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    pub const OFF: Self = Self(0);

    pub const fn percent(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    pub fn as_percent(self) -> u8 {
        self.0
    }
}

pub trait Light: Send {
    fn shine(&mut self, level: Level);
}

struct Lights {
    backlight: Box<dyn Light>,
    reading_lamp: Box<dyn Light>,
    last_touched: Option<Instant>,
    wanted: Level,
    backlight_level: Option<Level>,
    reading_lamp_level: Option<Level>,
}

#[derive(Clone)]
pub struct Lighting {
    lights: Arc<Mutex<Lights>>,
    settings: Settings,
}

impl Lighting {
    pub fn new(backlight: Box<dyn Light>, reading_lamp: Box<dyn Light>, settings: Settings) -> Self {
        let lights =
            Lights { backlight, reading_lamp, last_touched: None, wanted: Level::OFF, backlight_level: None, reading_lamp_level: None };
        Self { lights: Arc::new(Mutex::new(lights)), settings }
    }

    pub fn touched(&self, at: Instant) {
        self.lock().last_touched = Some(at);
        self.refresh(at);
    }

    pub fn shine_at_least(&self, level: Level) {
        self.lock().wanted = level;
    }

    pub fn refresh(&self, at: Instant) {
        let backlight_on = match self.settings.backlight() {
            BacklightDuration::Always => true,
            duration => {
                let lasts = duration.duration().unwrap_or_default();
                self.lock().last_touched.is_some_and(|touched| at.saturating_duration_since(touched) < lasts)
            }
        };
        let mut lights = self.lock();
        let backlight_level = lights.wanted.max(if backlight_on { BACKLIGHT_LEVEL } else { Level::OFF });
        let reading_lamp_level = lights.wanted.max(self.settings.reading_lamp().level());

        if lights.backlight_level != Some(backlight_level) {
            lights.backlight.shine(backlight_level);
            lights.backlight_level = Some(backlight_level);
        }
        if lights.reading_lamp_level != Some(reading_lamp_level) {
            lights.reading_lamp.shine(reading_lamp_level);
            lights.reading_lamp_level = Some(reading_lamp_level);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Lights> {
        self.lights.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::settings::{SettingsRecord, SettingsStore};

    #[derive(Clone, Default)]
    struct StubLight(Arc<Mutex<Vec<u8>>>);

    impl Light for StubLight {
        fn shine(&mut self, level: Level) {
            self.0.lock().unwrap().push(level.as_percent());
        }
    }

    impl StubLight {
        fn levels(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    struct StubSettingsStore;

    impl SettingsStore for StubSettingsStore {
        fn load(&mut self) -> Option<SettingsRecord> {
            None
        }
        fn save(&mut self, _: &SettingsRecord) {}
    }

    fn lighting() -> (Lighting, Settings, StubLight, StubLight) {
        let settings = Settings::load(Box::new(StubSettingsStore));
        let (backlight, lamp) = (StubLight::default(), StubLight::default());
        let lighting = Lighting::new(Box::new(backlight.clone()), Box::new(lamp.clone()), settings.clone());
        (lighting, settings, backlight, lamp)
    }

    #[test]
    fn the_backlight_lasts_as_long_as_the_setting_after_a_touch() {
        let (lighting, settings, backlight, _) = lighting();
        let start = Instant::now();
        lighting.touched(start);
        lighting.refresh(start + Duration::from_secs(9));
        assert_eq!(backlight.levels(), [20]);
        lighting.refresh(start + Duration::from_secs(10));
        assert_eq!(backlight.levels(), [20, 0]);
        assert_eq!(settings.backlight(), BacklightDuration::TenSeconds);
    }

    #[test]
    fn another_touch_keeps_the_backlight_on() {
        let (lighting, _, backlight, _) = lighting();
        let start = Instant::now();
        lighting.touched(start);
        lighting.touched(start + Duration::from_secs(8));
        lighting.refresh(start + Duration::from_secs(15));
        assert_eq!(backlight.levels(), [20]);
    }

    #[test]
    fn an_always_on_backlight_never_goes_out() {
        let (lighting, settings, backlight, _) = lighting();
        while settings.backlight() != BacklightDuration::Always {
            settings.choose_next_backlight();
        }
        lighting.refresh(Instant::now() + Duration::from_secs(3600));
        assert_eq!(backlight.levels(), [20]);
    }

    #[test]
    fn the_light_wanted_lights_both_lights_above_their_settings() {
        let (lighting, settings, backlight, lamp) = lighting();
        settings.choose_next_reading_lamp();
        let now = Instant::now();
        lighting.shine_at_least(Level::percent(5));
        lighting.refresh(now);
        assert_eq!((backlight.levels(), lamp.levels()), (vec![5], vec![10]));
        lighting.shine_at_least(Level::percent(60));
        lighting.refresh(now);
        assert_eq!((backlight.levels(), lamp.levels()), (vec![5, 60], vec![10, 60]));
        lighting.shine_at_least(Level::OFF);
        lighting.refresh(now);
        assert_eq!((backlight.levels(), lamp.levels()), (vec![5, 60, 0], vec![10, 60, 10]));
    }

    #[test]
    fn the_reading_lamp_shines_at_its_setting_and_is_set_only_on_change() {
        let (lighting, settings, _, lamp) = lighting();
        let now = Instant::now();
        lighting.refresh(now);
        lighting.refresh(now);
        settings.choose_next_reading_lamp();
        settings.choose_next_reading_lamp();
        lighting.refresh(now);
        lighting.refresh(now);
        assert_eq!(lamp.levels(), [0, 30]);
    }
}
