//! An example concept, standing in for the real ones until they arrive: something that
//! can be triggered, and remembers how often it was.

use crate::shared::Shared;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PingState {
    pub times_triggered: u32,
}

/// Every clone is the same Ping, whichever thread holds it.
#[derive(Clone, Debug, Default)]
pub struct Ping(Shared<PingState>);

impl Ping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger(&self) {
        self.0.update(|state| state.times_triggered = state.times_triggered.saturating_add(1));
    }

    pub fn state(&self) -> PingState {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn counts_every_trigger() {
        let ping = Ping::new();
        assert_eq!(ping.state().times_triggered, 0);
        ping.trigger();
        ping.trigger();
        assert_eq!(ping.state().times_triggered, 2);
    }

    #[test]
    fn clones_share_one_ping_across_threads() {
        let ping = Ping::new();
        let elsewhere = ping.clone();
        thread::spawn(move || elsewhere.trigger()).join().unwrap();
        assert_eq!(ping.state().times_triggered, 1);
    }
}
