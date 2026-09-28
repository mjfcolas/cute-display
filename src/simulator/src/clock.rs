use std::time::{SystemTime, UNIX_EPOCH};

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::Fault;

#[derive(Default)]
pub struct HostClock {
    ahead_seconds: i64,
}

fn computer_seconds() -> Result<i64, Fault> {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).map_err(Fault::new)?;
    i64::try_from(since_epoch.as_secs()).map_err(Fault::new)
}

impl RealTimeClock for HostClock {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        let time = DateTime::from_unix_seconds(computer_seconds()? + self.ahead_seconds);
        Ok(ClockReading { time, oscillator_stopped: false, alarm_raised: false })
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        self.ahead_seconds = time.unix_seconds() - computer_seconds()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_on_from_the_time_it_was_set() {
        let mut clock = HostClock::default();
        let set = DateTime { year: 2031, month: 1, day: 2, hour: 3, minute: 4, second: 5 };
        clock.set(set).unwrap();
        let read = clock.read().unwrap().time.unix_seconds();
        assert!((set.unix_seconds()..=set.unix_seconds() + 2).contains(&read));
    }
}
