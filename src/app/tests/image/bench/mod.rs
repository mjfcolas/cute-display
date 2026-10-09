//! What the tests run the image on, how they start it, and how they look at it.

mod hardware;
mod image;
mod setup;

pub use image::{eventually, let_the_speaker_settle, Image};
pub use setup::{RtcAtStart, Setup, ALARM_CONF, BREATHE_CONF, GENERAL_CONF, MINUTE, OPEN_METEO, RECORDED_FORECAST, SATURDAY_AT_SIX, SETTINGS_CONF, TIMER_CONF, ZEN_CONF};
