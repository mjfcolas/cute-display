use std::sync::{Arc, Mutex};

use hal::clock::{ClockReading, DateTime, RealTimeClock};
use hal::Fault;

/// One RTC for several owners: each clone reads and sets the same one, one at a time.
pub struct SharedRtc<C>(Arc<Mutex<C>>);

impl<C> SharedRtc<C> {
    pub fn new(rtc: C) -> Self {
        Self(Arc::new(Mutex::new(rtc)))
    }
}

impl<C> Clone for SharedRtc<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C: RealTimeClock> RealTimeClock for SharedRtc<C> {
    fn read(&mut self) -> Result<ClockReading, Fault> {
        self.0.lock().map_err(|_| Fault::new("the RTC was left mid-exchange"))?.read()
    }

    fn set(&mut self, time: DateTime) -> Result<(), Fault> {
        self.0.lock().map_err(|_| Fault::new("the RTC was left mid-exchange"))?.set(time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stopped(DateTime);
    impl RealTimeClock for Stopped {
        fn read(&mut self) -> Result<ClockReading, Fault> {
            Ok(ClockReading { time: self.0, oscillator_stopped: false, alarm_raised: false })
        }
        fn set(&mut self, time: DateTime) -> Result<(), Fault> {
            self.0 = time;
            Ok(())
        }
    }

    #[test]
    fn every_clone_reads_what_another_set() {
        let mut app = SharedRtc::new(Stopped(DateTime::from_unix_seconds(0)));
        let mut console = app.clone();
        let later = DateTime::from_unix_seconds(1_790_407_815);
        app.set(later).unwrap();
        assert_eq!(console.read().unwrap().time, later);
    }
}
