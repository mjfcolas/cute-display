//! A console on the serial line, or a Unix socket on the simulator, for a computer to
//! read the SD card and put files on it, since the card does not come out of the case,
//! and, for tests, to press the buttons and turn the wheel (`remote`), to see what the
//! lights and the speaker do (`observation`), and to read and set the RTC.
//!
//! Every line of it starts with `@@ <id>`; anything else on the line, the device's log
//! above all, is ignored by both ends. The other end is `tools/link/`.

pub mod console;
pub mod observation;
pub mod path;
pub mod protocol;
pub mod remote;

pub use console::MaintenanceConsole;
