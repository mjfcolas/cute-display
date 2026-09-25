//! The device is a set of apps, one of them in front.

use crate::shared::Shared;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    Counter,
    Echo,
    Ping,
}

/// Which app is in front. Every clone is the same; anything may bring an app to the
/// front, not only the person at the controls.
#[derive(Clone, Debug)]
pub struct Foreground(Shared<App>);

impl Foreground {
    pub fn new(app: App) -> Self {
        Self(Shared::new(app))
    }

    pub fn app(&self) -> App {
        self.0.get()
    }

    pub fn bring_to_front(&self, app: App) {
        self.0.update(|front| *front = app);
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn an_app_brought_to_the_front_is_in_front_for_everyone() {
        let foreground = Foreground::new(App::Counter);
        let service = foreground.clone();
        thread::spawn(move || service.bring_to_front(App::Ping)).join().unwrap();
        assert_eq!(foreground.app(), App::Ping);
    }
}
