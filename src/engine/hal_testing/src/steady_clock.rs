use core::time::Duration;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hal::steady::SteadyClock;

use crate::shared::lock;

/// Moves on only when slept on or advanced: time goes as the test says, never on its own.
#[derive(Clone)]
pub struct FakeSteadyClock(Arc<Mutex<Instant>>);

impl Default for FakeSteadyClock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }
}

impl FakeSteadyClock {
    pub fn advance(&self, by: Duration) {
        *lock(&self.0) += by;
    }
}

impl SteadyClock for FakeSteadyClock {
    fn now(&self) -> Instant {
        *lock(&self.0)
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}

/// Checks a clock against `SteadyClock`'s contract; it sleeps a few milliseconds of the
/// clock's time.
pub fn check_contract(clock: &impl SteadyClock) {
    let pause = Duration::from_millis(5);
    let before = clock.now();
    clock.sleep(pause);
    let after = clock.now();
    assert!(after >= before + pause, "a sleep lasts at least what was asked: {:?}", after - before);
    assert!(clock.now() >= after, "the time never goes back");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_steady_clock_keeps_the_contract() {
        check_contract(&FakeSteadyClock::default());
    }

    #[test]
    fn a_fake_steady_clock_moves_by_what_it_is_told_and_its_clones_with_it() {
        let clock = FakeSteadyClock::default();
        let clone = clock.clone();
        let start = clock.now();
        clock.advance(Duration::from_secs(60));
        clone.sleep(Duration::from_secs(1));
        assert_eq!(clock.now() - start, Duration::from_secs(61));
    }
}
