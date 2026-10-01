//! What the tests run the image on, start it with, and look at it through.

mod hardware;
mod image;
mod setup;

pub use image::{eventually, Image};
pub use setup::{RtcAtStart, Setup, ALARM_CONF, GENERAL_CONF, WIFI_CONF};
