use std::time::{SystemTime, UNIX_EPOCH};

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::Fault;

use crate::steady::ScaledClock;

/// The time it is in the simulated world: from where it started, on at the simulator's
/// speed.
#[derive(Clone, Copy)]
pub struct TrueTime {
    steady: ScaledClock,
    started_seconds: i64,
}

impl TrueTime {
    pub fn from_the_computer(steady: ScaledClock) -> Result<Self, Fault> {
        let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).map_err(Fault::new)?;
        Ok(Self::starting_at(steady, DateTime::from_unix_seconds(i64::try_from(since_epoch.as_secs()).map_err(Fault::new)?)))
    }

    pub fn starting_at(steady: ScaledClock, time: DateTime) -> Self {
        Self { steady, started_seconds: time.unix_seconds() }
    }

    pub fn unix_seconds(&self) -> i64 {
        self.started_seconds.saturating_add(i64::try_from(self.steady.elapsed().as_secs()).unwrap_or(i64::MAX))
    }
}

/// The RTC: the true time, ahead or behind it once set.
pub struct HostClock {
    time: TrueTime,
    ahead_seconds: i64,
}

impl HostClock {
    pub fn new(time: TrueTime) -> Self {
        Self { time, ahead_seconds: 0 }
    }
}

impl RealTimeClock for HostClock {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        let time = DateTime::from_unix_seconds(self.time.unix_seconds().saturating_add(self.ahead_seconds));
        Ok(ClockReading { time, oscillator_stopped: false, alarm_raised: false })
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        self.ahead_seconds = time.unix_seconds().saturating_sub(self.time.unix_seconds());
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
        clock::check_contract(&mut HostClock::new(TrueTime::from_the_computer(ScaledClock::new(NonZeroU32::MIN)).unwrap()));
    }

    #[test]
    fn runs_at_the_simulators_speed() {
        let steady = ScaledClock::new(NonZeroU32::new(1000).unwrap());
        let mut clock = HostClock::new(TrueTime::from_the_computer(steady).unwrap());
        clock.set(SET).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(clock.read().unwrap().time.unix_seconds() >= SET.unix_seconds() + 10);
    }

    #[test]
    fn starts_at_the_time_given() {
        let mut clock = HostClock::new(TrueTime::starting_at(ScaledClock::new(NonZeroU32::MIN), SET));
        let read = clock.read().unwrap().time.unix_seconds();
        assert!((SET.unix_seconds()..=SET.unix_seconds() + 1).contains(&read), "{read}");
    }
}
