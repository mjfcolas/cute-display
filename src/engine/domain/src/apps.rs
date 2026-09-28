//! The device is a set of apps, one of them in front. One is special: the system app,
//! which lists the others and holds the device's settings. It is not in the list, only
//! the system gesture reaches it, and leaving it goes back to where it was opened from.

use std::time::Instant;

use crate::clock::Clock;
use crate::files::Files;
use crate::internet::Internet;
use crate::lighting::Level;
use crate::shared::Shared;
use crate::sound::Sound;
use crate::time::LocalTime;

/// Which app, among those the image holds. Each app names itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AppId(&'static str);

impl AppId {
    /// The engine's own app.
    pub const SYSTEM: AppId = AppId("system");

    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub fn name(self) -> &'static str {
        self.0
    }
}

/// What an app runs whatever is on screen, called from the engine's threads. Every call
/// returns soon, but `fetch`.
pub trait AppService: Send + Sync {
    /// On the main thread, ten times a second. `local` is `None` until the clock is set.
    fn tick(&self, _now: Instant, _local: Option<LocalTime>) {}

    /// Whether the app has something to fetch now, on the network thread.
    fn fetch_due(&self, _now: Instant) -> bool {
        false
    }

    /// Fetches, once `fetch_due` said to, for as long as the network takes. The apps
    /// fetch one after another, so a fetch keeps its requests few and bounded, and holds
    /// nothing its screen waits for meanwhile.
    fn fetch(&self, _now: Instant) {}

    /// How bright the app wants the lights: they shine at least as bright as the app that
    /// wants the most.
    fn light(&self) -> Level {
        Level::OFF
    }
}

/// What the engine lends an app when it is installed: all the app gets from outside.
pub trait Services {
    fn foreground(&self) -> Foreground;
    fn clock(&self) -> Clock;
    fn internet(&self) -> Box<dyn Internet>;
    /// The app's own files, those named in `names` and no other.
    fn files(&self, names: &'static [&'static str]) -> Box<dyn Files>;
    fn sound(&self) -> Box<dyn Sound>;
}

#[derive(Clone, Copy, Debug)]
struct Front {
    app: AppId,
    /// Where leaving the system app goes back to.
    before_system: AppId,
}

/// Which app is in front. Every clone is the same; anything may bring an app to the
/// front, not only the person at the controls.
#[derive(Clone, Debug)]
pub struct Foreground(Shared<Front>);

impl Foreground {
    pub fn new(app: AppId) -> Self {
        Self(Shared::new(Front { app, before_system: app }))
    }

    pub fn app(&self) -> AppId {
        self.0.get().app
    }

    /// The app the system app goes back to.
    pub fn before_system(&self) -> AppId {
        self.0.get().before_system
    }

    pub fn bring_to_front(&self, app: AppId) {
        match app {
            AppId::SYSTEM => self.open_system(),
            app => self.0.update(|front| front.app = app),
        }
    }

    pub fn open_system(&self) {
        self.0.update(|front| {
            if front.app != AppId::SYSTEM {
                front.before_system = front.app;
                front.app = AppId::SYSTEM;
            }
        });
    }

    pub fn close_system(&self) {
        self.0.update(|front| {
            if front.app == AppId::SYSTEM {
                front.app = front.before_system;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    const WEATHER: AppId = AppId::new("weather");
    const RADAR: AppId = AppId::new("radar");

    #[test]
    fn an_app_brought_to_the_front_is_in_front_for_everyone() {
        let foreground = Foreground::new(WEATHER);
        let service = foreground.clone();
        thread::spawn(move || service.bring_to_front(RADAR)).join().unwrap();
        assert_eq!(foreground.app(), RADAR);
    }

    #[test]
    fn leaving_the_system_app_goes_back_where_it_was_opened_from() {
        let foreground = Foreground::new(RADAR);
        foreground.open_system();
        assert_eq!(foreground.app(), AppId::SYSTEM);
        foreground.open_system();
        foreground.close_system();
        assert_eq!(foreground.app(), RADAR);
    }

    #[test]
    fn choosing_an_app_from_the_system_app_leaves_it() {
        let foreground = Foreground::new(WEATHER);
        foreground.open_system();
        foreground.bring_to_front(RADAR);
        assert_eq!(foreground.app(), RADAR);
        foreground.open_system();
        assert_eq!(foreground.before_system(), RADAR);
    }

    #[test]
    fn leaving_the_system_app_it_started_on_stays_there() {
        let foreground = Foreground::new(AppId::SYSTEM);
        foreground.close_system();
        assert_eq!(foreground.app(), AppId::SYSTEM);
    }
}
