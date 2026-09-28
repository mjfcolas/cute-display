//! What every app relies on, and what the engine lends apps, in the product's own words.
//! Knows nothing of screens, controls or chips.

pub mod apps;
pub mod calendar;
pub mod clock;
pub mod fetch;
pub mod files;
pub mod internet;
pub mod lighting;
pub mod settings;
pub mod sound;
pub mod time;
pub mod time_zone;

mod shared;
