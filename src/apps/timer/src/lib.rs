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
use ::ui::{Installable, InstalledApp};

pub use crate::domain::timer::{
    Minutes, Mode, Ringer, Stretch, Timer, TimerSettings, TimerSettingsStore, TimerState, Volume, LONG_BREAK, RING_FOR, ROUNDS, SHORT_BREAK,
    WAKE_LIGHT_RISES_OVER,
};
pub use crate::ui::screen::TimerScreen;

use crate::infrastructure::chime_ringer::ChimeRinger;
use crate::infrastructure::timer_file::TimerFile;

pub const ID: AppId = AppId::new("timer");

pub const fn app<D: DrawTarget<Color = BinaryColor>>() -> Installable<D> {
    Installable { id: ID, title: "Timer", install: install::<D> }
}

fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let store = TimerFile::new(services.files());
    let ringer = ChimeRinger::new(services.sound());
    let timer = Timer::new(Box::new(store), Box::new(ringer), services.foreground());
    let screen = TimerScreen::new(timer.clone());
    InstalledApp { service: Arc::new(TimerService(timer)), screen: Box::new(screen) }
}

struct TimerService(Timer);

impl AppService for TimerService {
    fn tick(&self, now: Instant, _local: Option<LocalTime>) {
        self.0.tick(now);
    }

    fn light(&self) -> Level {
        self.0.wake_light()
    }
}
