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

#[cfg(test)]
mod tests {
    use hal_testing::steady_clock;

    use super::*;

    #[test]
    fn the_computers_clock_keeps_the_contract() {
        steady_clock::check_contract(&StdSteadyClock);
    }
}
