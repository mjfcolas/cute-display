//! The domain's contracts, implemented on the HAL.

pub mod conf_text;
pub mod hal_light;
pub mod internet;
pub mod location_file;
pub mod open_meteo;
pub mod settings_file;

#[cfg(test)]
mod test_storage;
