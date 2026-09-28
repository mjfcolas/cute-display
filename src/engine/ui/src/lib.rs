//! The presentation: the shell that hosts the app in front, the system app's screen, and
//! the toolkit apps draw with.
//!
//! It knows no hardware. Controls come in as a [`controls::ControlsSample`], and screens
//! go out through any embedded-graphics `DrawTarget`. Which app is in front belongs to the
//! domain; each app's state, to the app.

pub mod app_screen;
pub mod big_digits;
pub mod calendar_names;
pub mod controls;
pub mod gestures;
pub mod shell;
pub mod system;
pub mod text;

pub use app_screen::{AppScreen, Install, InstalledApp};
pub use shell::Shell;
