use std::sync::{Arc, Mutex};

use domain::clock::{Clock, TimeKeeper, TimeSource, TimeZoneSource};
use domain::fetch::Unavailable;
use domain::time::UtcTime;
use domain::time_zone::TimeZone;

use crate::shared::lock;

/// Keeps the time it was set to, without running on: a test moves it by setting it.
#[derive(Clone, Default)]
pub struct FakeTimeKeeper(Arc<Mutex<Option<UtcTime>>>);

impl FakeTimeKeeper {
    pub fn at(time: UtcTime) -> Self {
        Self(Arc::new(Mutex::new(Some(time))))
    }

    /// A clock on this keeper, with no network time, in UTC.
    pub fn offline_clock(&self) -> Clock {
        Clock::new(Box::new(self.clone()), Box::new(StubOfflineTimeSource), Box::new(StubTimeZoneSource(TimeZone::UTC)))
    }
}

impl TimeKeeper for FakeTimeKeeper {
    fn read(&mut self) -> Option<UtcTime> {
        *lock(&self.0)
    }

    fn set(&mut self, time: UtcTime) {
        *lock(&self.0) = Some(time);
    }
}

pub struct StubOfflineTimeSource;

impl TimeSource for StubOfflineTimeSource {
    fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
        Err(Unavailable("offline".into()))
    }
}

pub struct StubTimeZoneSource(pub TimeZone);

impl TimeZoneSource for StubTimeZoneSource {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
        Ok(Some(self.0))
    }
}

/// Checks a keeper, not yet set, against `TimeKeeper`'s contract.
pub fn check_keeper_contract(keeper: &mut impl TimeKeeper) {
    assert_eq!(keeper.read(), None, "a keeper never set does not know the time");
    let morning = UtcTime::from_unix_seconds(1_790_407_815);
    keeper.set(morning);
    assert_eq!(keeper.read(), Some(morning), "a keeper reads back the time it was set to");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_keeper_keeps_the_contract() {
        check_keeper_contract(&mut FakeTimeKeeper::default());
    }
}
