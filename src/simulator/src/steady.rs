use core::num::NonZeroU32;
use core::time::Duration;
use std::thread;
use std::time::Instant;

use hal::steady::SteadyClock;

/// The computer's steady time, `speed` times as fast from the start: the app image's
/// periods, the glass, the speaker and the RTC all run on it, the network does not.
#[derive(Clone, Copy)]
pub struct ScaledClock {
    started: Instant,
    speed: NonZeroU32,
}

impl ScaledClock {
    pub fn new(speed: NonZeroU32) -> Self {
        Self { started: Instant::now(), speed }
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed() * self.speed.get()
    }
}

impl SteadyClock for ScaledClock {
    fn now(&self) -> Instant {
        self.started + self.elapsed()
    }

    fn sleep(&self, duration: Duration) {
        thread::sleep(duration / self.speed.get());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_faster_clock_sleeps_less_and_moves_on_more() {
        let clock = ScaledClock::new(NonZeroU32::new(50).unwrap());
        let real = Instant::now();
        let before = clock.now();
        clock.sleep(Duration::from_secs(1));
        assert!(real.elapsed() < Duration::from_millis(500));
        assert!(clock.now().duration_since(before) >= Duration::from_secs(1));
    }
}
