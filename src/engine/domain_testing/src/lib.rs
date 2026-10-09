//! For tests only: test doubles of the contracts that the domain needs from the outside
//! world, and the checks that every implementation must pass. So a test on a double shows
//! what the infrastructure would do.
//!
//! A double that a test acts on or looks at is a handle: its clones share one state.

pub mod files;
pub mod internet;
pub mod place;
pub mod settings;
pub mod time;

mod shared;
