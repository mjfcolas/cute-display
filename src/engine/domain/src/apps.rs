use std::time::Instant;

use crate::clock::Clock;
use crate::files::Files;
use crate::internet::Internet;
use crate::lighting::Level;
use crate::place::PlaceSource;
use crate::shared::Shared;
use crate::sound::Sound;
use crate::time::LocalTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AppId(&'static str);

impl AppId {
    pub const SYSTEM: AppId = AppId("system");

    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub fn name(self) -> &'static str {
        self.0
    }
}

pub fn chosen(image: &[AppId], wanted: Option<&[String]>) -> Vec<AppId> {
    let Some(wanted) = wanted else {
        return image.to_vec();
    };
    let mut chosen: Vec<AppId> = Vec::new();
    for name in wanted {
        if let Some(&app) = image.iter().find(|app| app.name() == name) {
            if !chosen.contains(&app) {
                chosen.push(app);
            }
        }
    }
    chosen
}

pub trait AppService: Send + Sync {
    fn tick(&self, _now: Instant, _local: Option<LocalTime>) {}

    fn fetch_due(&self, _now: Instant) -> bool {
        false
    }

    /// The apps fetch one after another, so a fetch keeps its requests few and bounded,
    /// and holds nothing its screen waits for meanwhile.
    fn fetch(&self, _now: Instant) {}

    fn light(&self) -> Level {
        Level::OFF
    }
}

pub trait Services {
    fn foreground(&self) -> Foreground;
    fn clock(&self) -> Clock;
    fn internet(&self) -> Box<dyn Internet>;
    fn place(&self) -> Box<dyn PlaceSource>;
    fn files(&self) -> Box<dyn Files>;
    fn sound(&self) -> Box<dyn Sound>;
}

#[derive(Clone, Copy, Debug)]
struct Front {
    app: AppId,
    before_system: AppId,
}

#[derive(Clone, Debug)]
pub struct Foreground(Shared<Front>);

impl Foreground {
    pub fn new(app: AppId) -> Self {
        Self(Shared::new(Front { app, before_system: app }))
    }

    pub fn app(&self) -> AppId {
        self.0.get().app
    }

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

    #[test]
    fn without_a_choice_every_app_of_the_image_runs() {
        assert_eq!(chosen(&[WEATHER, RADAR], None), [WEATHER, RADAR]);
    }

    #[test]
    fn the_apps_chosen_run_in_the_order_chosen_once_each_if_the_image_holds_them() {
        let wanted = ["radar", "alarm", "weather", "radar"].map(String::from);
        assert_eq!(chosen(&[WEATHER, RADAR], Some(&wanted)), [RADAR, WEATHER]);
        assert_eq!(chosen(&[WEATHER, RADAR], Some(&[])), []);
    }
}
