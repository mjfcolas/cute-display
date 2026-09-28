//! The alarm clock app: a wake-up time for each day, kept in `cute-display/alarm.conf`.
//! The lights rise before it like the sun; it rings, coming to the front whatever app is
//! there.

mod domain;
mod infrastructure;
mod ui;

use std::sync::Arc;
use std::time::Instant;

use ::domain::apps::{AppId, AppService, Services};
use ::domain::lighting::Level;
use ::domain::time::LocalTime;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use forecast::Weather;
use ::ui::InstalledApp;

pub use crate::domain::alarm_clock::{AlarmClock, AlarmSchedule, AlarmScheduleStore, AlarmState, Ringer, Volume, RING_FOR, SNOOZE, SUNRISE};
pub use crate::ui::screen::AlarmScreen;

use crate::infrastructure::alarm_file::{AlarmFile, ALARM_FILE};
use crate::infrastructure::ringtone::SoundRinger;

pub const ID: AppId = AppId::new("alarm");

pub fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let store = AlarmFile::new(services.files(&[ALARM_FILE]));
    let alarm = AlarmClock::new(Box::new(store), Box::new(SoundRinger::new(services.sound())), services.foreground());
    let weather = Weather::from_open_meteo(services);
    let screen = AlarmScreen::new(alarm.clone(), services.clock(), weather.clone());
    InstalledApp { service: Arc::new(AlarmWithForecast { alarm, weather }), screen: Box::new(screen) }
}

struct AlarmWithForecast {
    alarm: AlarmClock,
    weather: Weather,
}

impl AppService for AlarmWithForecast {
    fn tick(&self, _now: Instant, local: Option<LocalTime>) {
        if let Some(local) = local {
            self.alarm.tick(local);
        }
    }

    fn fetch_due(&self, now: Instant) -> bool {
        self.weather.fetch_due(now)
    }

    fn fetch(&self, now: Instant) {
        self.weather.fetch(now);
    }

    fn light(&self) -> Level {
        self.alarm.sunrise()
    }
}
