//! For tests only: test doubles of the contracts the domain needs from the outside world,
//! and the checks every implementation of one passes, so that a test on a double says
//! what the infrastructure would do.
//!
//! A double that a test acts on or looks into is a handle: its clones share one state.

pub mod files;
pub mod internet;
pub mod place;
pub mod settings;
pub mod time;

mod shared;
