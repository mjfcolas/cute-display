//! The device is a set of apps, one of them in front. One is special: the system app,
//! which lists the others and holds the device's settings. It is not in the list, only
//! the system gesture reaches it, and leaving it goes back to where it was opened from.

use crate::shared::Shared;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    System,
    Weather,
    Radar,
}

impl App {
    /// Every app but the system one, in the order they are offered.
    pub const LAUNCHABLE: [App; 2] = [App::Weather, App::Radar];
}

#[derive(Clone, Copy, Debug)]
struct Front {
    app: App,
    /// Where leaving the system app goes back to.
    before_system: App,
}

/// Which app is in front. Every clone is the same; anything may bring an app to the
/// front, not only the person at the controls.
#[derive(Clone, Debug)]
pub struct Foreground(Shared<Front>);

impl Foreground {
    pub fn new(app: App) -> Self {
        let before_system = if app == App::System { App::Weather } else { app };
        Self(Shared::new(Front { app, before_system }))
    }

    pub fn app(&self) -> App {
        self.0.get().app
    }

    /// The app the system app goes back to.
    pub fn before_system(&self) -> App {
        self.0.get().before_system
    }

    pub fn bring_to_front(&self, app: App) {
        match app {
            App::System => self.open_system(),
            app => self.0.update(|front| front.app = app),
        }
    }

    pub fn open_system(&self) {
        self.0.update(|front| {
            if front.app != App::System {
                front.before_system = front.app;
                front.app = App::System;
            }
        });
    }

    pub fn close_system(&self) {
        self.0.update(|front| {
            if front.app == App::System {
                front.app = front.before_system;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn an_app_brought_to_the_front_is_in_front_for_everyone() {
        let foreground = Foreground::new(App::Weather);
        let service = foreground.clone();
        thread::spawn(move || service.bring_to_front(App::Radar)).join().unwrap();
        assert_eq!(foreground.app(), App::Radar);
    }

    #[test]
    fn leaving_the_system_app_goes_back_where_it_was_opened_from() {
        let foreground = Foreground::new(App::Radar);
        foreground.open_system();
        assert_eq!(foreground.app(), App::System);
        foreground.open_system();
        foreground.close_system();
        assert_eq!(foreground.app(), App::Radar);
    }

    #[test]
    fn choosing_an_app_from_the_system_app_leaves_it() {
        let foreground = Foreground::new(App::Weather);
        foreground.open_system();
        foreground.bring_to_front(App::Radar);
        assert_eq!(foreground.app(), App::Radar);
        foreground.open_system();
        assert_eq!(foreground.before_system(), App::Radar);
    }

    #[test]
    fn the_system_app_is_not_among_the_apps_offered() {
        assert!(!App::LAUNCHABLE.contains(&App::System));
    }
}
