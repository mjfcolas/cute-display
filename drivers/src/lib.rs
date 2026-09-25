//! The HAL implemented on the chips of the Habity board (ESP32-S3).
//!
//! Chip logic that needs no ESP32 compiles everywhere and is tested on the host; the
//! rest only exists when building for `espidf`. Which chip sits on which pin is the
//! firmware's `board`, not this crate.

pub mod button;
pub mod ds3231;
pub mod uc8253;

#[cfg(target_os = "espidf")]
mod or_fault;

#[cfg(target_os = "espidf")]
pub mod adc_power;
#[cfg(target_os = "espidf")]
pub mod esp_http;
#[cfg(target_os = "espidf")]
pub mod esp_system;
#[cfg(target_os = "espidf")]
pub mod esp_wifi;
#[cfg(target_os = "espidf")]
pub mod i2c;
#[cfg(target_os = "espidf")]
pub mod i2s_speaker;
#[cfg(target_os = "espidf")]
pub mod ledc_light;
#[cfg(target_os = "espidf")]
pub mod pcnt_encoder;
#[cfg(target_os = "espidf")]
pub mod sdmmc_card;
#[cfg(target_os = "espidf")]
pub mod usb_console;
