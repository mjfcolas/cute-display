//! A console on the serial line, or a Unix socket on the simulator, for a computer to
//! read the SD card and put files on it, since the card does not come out of the case,
//! and to press the buttons and turn the wheel, for tests (`remote`).
//!
//! Every line of it starts with `@@ <id>`; anything else on the line, the device's log
//! above all, is ignored by both ends. The other end is `tools/link/`.

pub mod console;
pub mod path;
pub mod protocol;
pub mod remote;

pub use console::MaintenanceConsole;
