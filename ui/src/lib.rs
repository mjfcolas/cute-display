//! The presentation: a screen per app, the system app's among them.
//!
//! It knows no hardware. Controls come in as a [`controls::ControlsSample`], and screens
//! go out through any embedded-graphics `DrawTarget`. Which app is in front, and every
//! app's state, belong to the domain.

pub mod app_screen;
pub mod apps;
pub mod controls;
pub mod gestures;
pub mod shell;

mod text;

pub use app_screen::AppScreen;
pub use shell::{ScreenChange, Shell};
