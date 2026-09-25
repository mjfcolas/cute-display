//! UltraChip UC8253, the controller of the GDEY037T03 e-paper panel.
//!
//! No datasheet for it was found: everything here was measured on the device.

pub mod memory;

#[cfg(target_os = "espidf")]
mod controller;

#[cfg(target_os = "espidf")]
pub use controller::Uc8253;
