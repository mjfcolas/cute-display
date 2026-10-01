//! What the tests run the image on, start it with, and look at it through.

mod hardware;
mod image;
mod setup;

pub use image::{eventually, let_the_speaker_settle, Image};
pub use setup::{RtcAtStart, Setup, ALARM_CONF, GENERAL_CONF, MINUTE, OPEN_METEO, RECORDED_FORECAST, SATURDAY_AT_SIX, SETTINGS_CONF};
