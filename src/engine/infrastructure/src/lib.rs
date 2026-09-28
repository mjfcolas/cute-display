//! The domain's contracts, implemented on the HAL.

pub mod adsb_fi;
pub mod alarm_file;
pub mod airports_file;
pub mod conf_text;
pub mod hal_light;
pub mod internet;
pub mod ntp;
pub mod open_meteo;
pub mod place_file;
pub mod rtc_keeper;
pub mod settings_file;
pub mod speaker_ringer;
pub mod time_zone_file;

#[cfg(test)]
mod test_storage;
