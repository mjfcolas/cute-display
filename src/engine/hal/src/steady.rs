use core::time::Duration;
use std::time::Instant;

/// The time that the app image runs on: its periods, its timeouts, how long a gesture
/// lasts. It never goes back. On a computer, it may run faster than real time.
pub trait SteadyClock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}
