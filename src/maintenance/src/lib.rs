//! A console on the serial line, or on a Unix socket in the simulator. A computer uses it
//! to read the SD card and put files on it, because the card cannot be taken out of the
//! case. Tests also use it to press the buttons and turn the wheel (`remote`), to see what
//! the screen, the lights and the speaker do (`observation`), and to read and set the RTC.
//!
//! Each line of the protocol starts with `@@ <id>`. Both ends ignore anything else on the
//! line, mainly the device's log. The other end is `tools/link/`.

pub mod console;
#[cfg(test)]
mod fake_image_controls;
pub mod observation;
pub mod path;
pub mod protocol;
pub mod remote;

pub use console::MaintenanceConsole;
