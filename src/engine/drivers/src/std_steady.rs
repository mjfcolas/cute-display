use core::time::Duration;
use std::thread;
use std::time::Instant;

use hal::steady::SteadyClock;

#[derive(Clone, Copy, Default)]
pub struct StdSteadyClock;

impl SteadyClock for StdSteadyClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}
