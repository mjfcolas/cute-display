//! The domain's contracts, implemented on the HAL.

pub mod adsb_fi;
pub mod airports_file;
pub mod conf_text;
pub mod hal_light;
pub mod internet;
pub mod open_meteo;
pub mod place_file;
pub mod settings_file;

#[cfg(test)]
mod test_storage;
