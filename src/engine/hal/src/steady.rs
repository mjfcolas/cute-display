use core::time::Duration;
use std::time::Instant;

/// The time the app image runs on: its periods, its timeouts, how long a gesture lasts.
/// Never goes back; on a computer it may run faster than the wall's.
pub trait SteadyClock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}
