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

pub use crate::domain::zen_sessions::{
    Player, SessionLength, Soundscape, Volume, ZenSessions, ZenSettings, ZenSettingsStore, ZenState, DUSK, FADE_IN, FADE_OUT,
};
pub use crate::ui::screen::ZenScreen;

use crate::infrastructure::sound_player::SoundPlayer;
use crate::infrastructure::zen_file::ZenFile;

pub const ID: AppId = AppId::new("zen");

pub const fn app<D: DrawTarget<Color = BinaryColor>>() -> Installable<D> {
    Installable { id: ID, title: "Zen", install: install::<D> }
}

fn install<D: DrawTarget<Color = BinaryColor>>(services: &dyn Services) -> InstalledApp<D> {
    let store = ZenFile::new(services.files());
    let player = SoundPlayer::new(services.sound(), services.files());
    let zen = ZenSessions::new(Box::new(store), Box::new(player), services.foreground());
    let screen = ZenScreen::new(zen.clone(), services.clock());
    InstalledApp { service: Arc::new(ZenService(zen)), screen: Box::new(screen) }
}

struct ZenService(ZenSessions);

impl AppService for ZenService {
    fn tick(&self, now: Instant, local: Option<LocalTime>) {
        self.0.tick(now, local);
    }

    fn light(&self) -> Level {
        self.0.dusk()
    }
}
