use std::sync::{Arc, Mutex};

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::Fault;

use crate::shared::lock;

/// What a DS3231 holds at power-up: its registers' zero.
pub(crate) const POWER_UP: DateTime = DateTime { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 0 };

/// An RTC that stands at the time it was set to: a test moves it by setting it.
#[derive(Clone)]
pub struct FakeRtc(Arc<Mutex<Rtc>>);

struct Rtc {
    time: Option<DateTime>,
    fault: Option<Fault>,
}

impl FakeRtc {
    pub fn at(time: DateTime) -> Self {
        Self::new(Rtc { time: Some(time), fault: None })
    }

    /// Never set since its battery ran out: its oscillator is stopped.
    pub fn stopped() -> Self {
        Self::new(Rtc { time: None, fault: None })
    }

    /// Nothing answers on the bus.
    pub fn unreadable(reason: &str) -> Self {
        let rtc = Self::stopped();
        rtc.fail_with(reason);
        rtc
    }

    fn new(rtc: Rtc) -> Self {
        Self(Arc::new(Mutex::new(rtc)))
    }

    /// From now on, reading and setting fail with `reason`.
    pub fn fail_with(&self, reason: &str) {
        lock(&self.0).fault = Some(Fault::new(reason));
    }

    /// `None` while the oscillator is stopped.
    pub fn time(&self) -> Option<DateTime> {
        lock(&self.0).time
    }
}

impl RealTimeClock for FakeRtc {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        let rtc = lock(&self.0);
        if let Some(fault) = &rtc.fault {
            return Err(fault.clone());
        }
        Ok(ClockReading { time: rtc.time.unwrap_or(POWER_UP), oscillator_stopped: rtc.time.is_none(), alarm_raised: false })
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        let mut rtc = lock(&self.0);
        if let Some(fault) = &rtc.fault {
            return Err(fault.clone());
        }
        rtc.time = Some(time);
        Ok(())
    }
}

/// Checks an RTC against `RealTimeClock`'s contract. It is set, so a real one may be a
/// second or two on when read back.
pub fn check_contract(rtc: &mut impl RealTimeClock) {
    let time = DateTime { year: 2026, month: 9, day: 26, hour: 7, minute: 30, second: 15 };
    assert_eq!(rtc.set(time), Ok(()));
    let reading = rtc.read();
    let seconds = time.unix_seconds();
    assert!(
        reading.as_ref().is_ok_and(|r| !r.oscillator_stopped && (seconds..=seconds + 2).contains(&r.time.unix_seconds())),
        "the time set is read back, the oscillator running: {reading:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_rtc_keeps_the_contract() {
        check_contract(&mut FakeRtc::stopped());
    }

    #[test]
    fn a_stopped_rtc_says_so_until_set() {
        let mut rtc = FakeRtc::stopped();
        assert!(rtc.read().unwrap().oscillator_stopped);
        rtc.set(POWER_UP).unwrap();
        assert!(!rtc.read().unwrap().oscillator_stopped);
    }

    #[test]
    fn an_unreadable_rtc_fails_both_ways() {
        let mut rtc = FakeRtc::unreadable("I2C: no answer");
        assert_eq!(rtc.read(), Err(Fault::new("I2C: no answer")));
        assert_eq!(rtc.set(POWER_UP), Err(Fault::new("I2C: no answer")));
    }
}
