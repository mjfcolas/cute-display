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

pub use crate::domain::breathing::{
    Breath, Breathing, BreathingSettings, BreathingSettingsStore, BreathingState, Fullness, Rhythm, Step, Volume, BREATH_LIGHT,
};
pub use crate::ui::screen::BreatheScreen;

use crate::infrastructure::breathe_file::BreatheFile;
use crate::infrastructure::noise_breath::NoiseBreath;

pub const ID: AppId = AppId::new("breathe");

pub const fn app<D: DrawTarget<Color = BinaryColor>>() -> Installable<D> {
    Installable { id: ID, title: "Breathe", install: install::<D> }
}

fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let breathing = Breathing::new(Box::new(BreatheFile::new(services.files())), Box::new(NoiseBreath::new(services.sound())));
    let screen = BreatheScreen::new(breathing.clone());
    InstalledApp { service: Arc::new(BreatheService(breathing)), screen: Box::new(screen) }
}

struct BreatheService(Breathing);

impl AppService for BreatheService {
    fn tick(&self, now: Instant, _local: Option<LocalTime>) {
        self.0.tick(now);
    }

    fn light(&self) -> Level {
        self.0.light()
    }
}
