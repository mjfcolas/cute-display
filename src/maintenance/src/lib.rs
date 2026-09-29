//! A console on the serial line, for a computer to read the SD card and put files on it,
//! since the card does not come out of the case, and to press the buttons and turn the
//! wheel, for tests (`remote`).
//!
//! Every line of it starts with `@@ <id>`; anything else on the line, the device's log
//! above all, is ignored by both ends. For the card, the other end is the installer's
//! `tools/installer/src/cute_display_installer/card/console.py`.

pub mod console;
pub mod path;
pub mod protocol;
pub mod remote;

pub use console::MaintenanceConsole;
