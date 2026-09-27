//! A console on the serial line, for a computer to read the SD card and put files on it:
//! the card does not come out of the case.
//!
//! Every line of it starts with `@@ <id>`; anything else on the line, the device's log
//! above all, is ignored by both ends. The other end is the installer's
//! `tools/installer/src/cute_display_installer/card/console.py`.

pub mod console;
pub mod path;
pub mod protocol;

pub use console::MaintenanceConsole;
