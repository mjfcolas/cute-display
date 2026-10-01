use std::sync::{Arc, Mutex};
use std::time::Instant;

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::steady::SteadyClock;
use hal::Fault;

use crate::clock::POWER_UP;
use crate::shared::lock;

/// An RTC whose time runs on `steady` from where it was set, as a battery-backed chip's
/// does: time a test moves on passes on it too.
#[derive(Clone)]
pub struct FakeRunningRtc<S> {
    steady: S,
    /// `None` while the oscillator is stopped.
    setting: Arc<Mutex<Option<Setting>>>,
}

#[derive(Clone, Copy)]
struct Setting {
    time: DateTime,
    at: Instant,
}

impl<S: SteadyClock> FakeRunningRtc<S> {
    pub fn at(time: DateTime, steady: S) -> Self {
        let setting = Some(Setting { time, at: steady.now() });
        Self { steady, setting: Arc::new(Mutex::new(setting)) }
    }

    /// Never set since its battery ran out: its oscillator is stopped.
    pub fn stopped(steady: S) -> Self {
        Self { steady, setting: Arc::default() }
    }

    pub fn time(&self) -> Option<DateTime> {
        let setting = *lock(&self.setting);
        setting.map(|Setting { time, at }| {
            let run = self.steady.now().saturating_duration_since(at).as_secs();
            DateTime::from_unix_seconds(time.unix_seconds().saturating_add(i64::try_from(run).unwrap_or(i64::MAX)))
        })
    }
}

impl<S: SteadyClock> RealTimeClock for FakeRunningRtc<S> {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        let time = self.time();
        Ok(ClockReading { time: time.unwrap_or(POWER_UP), oscillator_stopped: time.is_none(), alarm_raised: false })
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        *lock(&self.setting) = Some(Setting { time, at: self.steady.now() });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::clock::check_contract;
    use crate::steady_clock::FakeSteadyClock;

    #[test]
    fn a_running_rtc_keeps_the_contract() {
        check_contract(&mut FakeRunningRtc::stopped(FakeSteadyClock::default()));
    }

    #[test]
    fn a_running_rtc_never_set_says_its_oscillator_stopped() {
        assert!(FakeRunningRtc::stopped(FakeSteadyClock::default()).read().unwrap().oscillator_stopped);
    }

    #[test]
    fn a_running_rtc_runs_on_its_steady_clock() {
        let steady = FakeSteadyClock::default();
        let mut rtc = FakeRunningRtc::stopped(steady.clone());
        rtc.set(POWER_UP).unwrap();
        steady.advance(Duration::from_secs(90));
        assert_eq!(rtc.time().map(DateTime::unix_seconds), Some(POWER_UP.unix_seconds() + 90));
    }
}
