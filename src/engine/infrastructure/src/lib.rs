//! The domain's contracts, implemented on the HAL.

pub mod card_files;
mod conf_file;
pub mod hal_light;
pub mod internet;
pub mod ntp;
pub mod rtc_keeper;
pub mod settings_file;
pub mod speaker_sound;
pub mod time_zone_file;

#[cfg(test)]
mod test_storage;
