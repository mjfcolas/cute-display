use std::time::{SystemTime, UNIX_EPOCH};

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::Fault;

use crate::steady::ScaledClock;

/// The computer's time when the simulator started, run on from there at its speed.
pub struct HostClock {
    steady: ScaledClock,
    started_seconds: i64,
    ahead_seconds: i64,
}

impl HostClock {
    pub fn new(steady: ScaledClock) -> Result<Self, Fault> {
        let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).map_err(Fault::new)?;
        let started_seconds = i64::try_from(since_epoch.as_secs()).map_err(Fault::new)?;
        Ok(Self { steady, started_seconds, ahead_seconds: 0 })
    }

    fn seconds(&self) -> i64 {
        self.started_seconds.saturating_add(i64::try_from(self.steady.elapsed().as_secs()).unwrap_or(i64::MAX))
    }
}

impl RealTimeClock for HostClock {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        let time = DateTime::from_unix_seconds(self.seconds().saturating_add(self.ahead_seconds));
        Ok(ClockReading { time, oscillator_stopped: false, alarm_raised: false })
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        self.ahead_seconds = time.unix_seconds().saturating_sub(self.seconds());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use hal_testing::clock;

    use super::*;

    const SET: DateTime = DateTime { year: 2031, month: 1, day: 2, hour: 3, minute: 4, second: 5 };

    #[test]
    fn a_host_clock_keeps_the_contract() {
        clock::check_contract(&mut HostClock::new(ScaledClock::new(NonZeroU32::MIN)).unwrap());
    }

    #[test]
    fn runs_at_the_simulators_speed() {
        let steady = ScaledClock::new(NonZeroU32::new(1000).unwrap());
        let mut clock = HostClock::new(steady).unwrap();
        clock.set(SET).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(clock.read().unwrap().time.unix_seconds() >= SET.unix_seconds() + 10);
    }
}
