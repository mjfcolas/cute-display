//! The board's real-time clock keeps UTC.

use domain::calendar::Date;
use domain::clock::TimeKeeper;
use domain::time::{LocalTime, TimeOfDay, UtcOffset, UtcTime};
use hal::clock::{DateTime, RealTimeClock};
use hal::Fault;

pub struct RtcKeeper<C> {
    clock: C,
    /// What went wrong at the last read, so that a lasting fault is logged once.
    fault: Option<Fault>,
}

impl<C: RealTimeClock> RtcKeeper<C> {
    pub fn new(clock: C) -> Self {
        Self { clock, fault: None }
    }

    fn note(&mut self, fault: Option<Fault>) {
        if fault != self.fault {
            if let Some(fault) = &fault {
                log::warn!("clock: {fault}");
            }
            self.fault = fault;
        }
    }
}

impl<C: RealTimeClock + Send> TimeKeeper for RtcKeeper<C> {
    fn read(&mut self) -> Option<UtcTime> {
        let reading = self.clock.read().and_then(|reading| {
            if reading.oscillator_stopped {
                return Err(Fault::new("the RTC stopped and lost the time"));
            }
            utc(reading.time).ok_or_else(|| Fault::new(format!("the RTC reads an impossible time: {:?}", reading.time)))
        });
        self.note(reading.as_ref().err().cloned());
        reading.ok()
    }

    fn set(&mut self, time: UtcTime) {
        let set_to = time.at_offset(UtcOffset::ZERO);
        let (date, time_of_day) = (set_to.date, set_to.time_of_day);
        let registers = DateTime {
            year: date.year(),
            month: date.month(),
            day: date.day(),
            hour: time_of_day.hour(),
            minute: time_of_day.minute(),
            second: set_to.second,
        };
        match self.clock.set(registers) {
            Ok(()) => log::info!(
                "clock: set to {}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
                date.year(),
                date.month(),
                date.day(),
                time_of_day.hour(),
                time_of_day.minute(),
                set_to.second
            ),
            Err(fault) => log::warn!("clock: {fault}; not set"),
        }
    }
}

fn utc(time: DateTime) -> Option<UtcTime> {
    let date = Date::new(time.year, time.month, time.day)?;
    let time_of_day = TimeOfDay::new(time.hour, time.minute)?;
    let second = Some(time.second).filter(|&s| s < 60)?;
    Some(UtcTime::from_unix_seconds(LocalTime { date, time_of_day, second }.seconds_since_epoch()))
}

#[cfg(test)]
mod tests {
    use domain_testing::time;
    use hal_testing::clock::FakeRtc;

    use super::*;

    #[test]
    fn a_keeper_on_the_rtc_keeps_the_contract() {
        time::check_keeper_contract(&mut RtcKeeper::new(FakeRtc::stopped()));
    }

    #[test]
    fn what_is_set_is_read_back_as_utc() {
        let rtc = FakeRtc::stopped();
        let mut keeper = RtcKeeper::new(rtc.clone());
        let morning = UtcTime::from_unix_seconds(1_790_407_815);
        keeper.set(morning);
        assert_eq!(rtc.time(), Some(DateTime { year: 2026, month: 9, day: 26, hour: 7, minute: 30, second: 15 }));
        assert_eq!(keeper.read(), Some(morning));
    }

    #[test]
    fn an_impossible_time_is_no_time() {
        let mut keeper = RtcKeeper::new(FakeRtc::at(DateTime { year: 2026, month: 4, day: 31, hour: 7, minute: 0, second: 0 }));
        assert_eq!(keeper.read(), None);
    }

    #[test]
    fn a_stopped_or_unreadable_clock_does_not_know_the_time() {
        let rtc = FakeRtc::stopped();
        let mut keeper = RtcKeeper::new(rtc.clone());
        assert_eq!(keeper.read(), None);
        rtc.fail_with("I2C read");
        assert_eq!(keeper.read(), None);
        assert_eq!(keeper.fault, Some(Fault::new("I2C read")));
    }
}
