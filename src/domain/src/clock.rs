//! What time it is. A keeper that runs on its own, battery-backed, holds UTC; a source
//! out in the world sets it right once a day; the time zone turns it into the time on
//! the wall.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::fetch::Unavailable;
use crate::shared::Shared;
use crate::time::{LocalTime, UtcTime};
use crate::time_zone::TimeZone;

pub const SYNC_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
pub const RETRY_SYNC_AFTER: Duration = Duration::from_secs(10 * 60);
/// How soon a change of time zone shows.
pub const READ_ZONE_EVERY: Duration = Duration::from_secs(60);

/// Keeps the time while the device is off. A keeper that fails says so itself.
pub trait TimeKeeper: Send {
    /// `None` when it cannot tell, or has lost the time.
    fn read(&mut self) -> Option<UtcTime>;
    fn set(&mut self, time: UtcTime);
}

/// Knows the time for sure, but has to be asked from afar.
pub trait TimeSource: Send {
    fn fetch(&mut self) -> Result<UtcTime, Unavailable>;
}

pub trait TimeZoneSource: Send {
    /// `None` when none was chosen, or it cannot be read.
    fn time_zone(&mut self) -> Option<TimeZone>;
}

#[derive(Clone, Copy)]
enum SyncAttempt {
    Succeeded(Instant),
    Failed(Instant),
}

#[derive(Clone, Copy)]
struct State {
    time: Option<UtcTime>,
    zone: TimeZone,
    zone_read: Option<Instant>,
    last_sync: Option<SyncAttempt>,
}

/// Every clone is the same clock.
#[derive(Clone)]
pub struct Clock {
    state: Shared<State>,
    keeper: Arc<Mutex<Box<dyn TimeKeeper>>>,
    source: Arc<Mutex<Box<dyn TimeSource>>>,
    zones: Arc<Mutex<Box<dyn TimeZoneSource>>>,
}

impl Clock {
    pub fn new(keeper: Box<dyn TimeKeeper>, source: Box<dyn TimeSource>, zones: Box<dyn TimeZoneSource>) -> Self {
        Self {
            state: Shared::new(State { time: None, zone: TimeZone::default(), zone_read: None, last_sync: None }),
            keeper: Arc::new(Mutex::new(keeper)),
            source: Arc::new(Mutex::new(source)),
            zones: Arc::new(Mutex::new(zones)),
        }
    }

    /// The time on the wall when the keeper was last read; `None` while nobody knows it.
    pub fn now(&self) -> Option<LocalTime> {
        let state = self.state.get();
        state.time.map(|time| state.zone.local(time))
    }

    /// Reads the keeper, and the time zone every [`READ_ZONE_EVERY`].
    pub fn tick(&self, now: Instant) {
        let time = lock(&self.keeper).read();
        let zone_read = self.state.get().zone_read;
        let zone = zone_read
            .is_none_or(|read| now.saturating_duration_since(read) >= READ_ZONE_EVERY)
            .then(|| lock(&self.zones).time_zone().unwrap_or_default());
        self.state.update(|state| {
            state.time = time;
            if let Some(zone) = zone {
                state.zone = zone;
                state.zone_read = Some(now);
            }
        });
    }

    pub fn is_sync_due(&self, now: Instant) -> bool {
        match self.state.get().last_sync {
            None => true,
            Some(SyncAttempt::Succeeded(at)) => now.saturating_duration_since(at) >= SYNC_EVERY,
            Some(SyncAttempt::Failed(at)) => now.saturating_duration_since(at) >= RETRY_SYNC_AFTER,
        }
    }

    /// Asks the source for the time if it is due, and sets the keeper to it. Blocks for
    /// as long as the source takes, but never holds up whoever reads the time meanwhile.
    pub fn sync_if_due(&self, now: Instant) -> Result<(), Unavailable> {
        if !self.is_sync_due(now) {
            return Ok(());
        }
        let fetched = lock(&self.source).fetch();
        let attempt = match fetched {
            Ok(time) => {
                lock(&self.keeper).set(time);
                self.state.update(|state| state.time = Some(time));
                SyncAttempt::Succeeded(now)
            }
            Err(_) => SyncAttempt::Failed(now),
        };
        self.state.update(|state| state.last_sync = Some(attempt));
        fetched.map(|_| ())
    }
}

fn lock<T: ?Sized>(mutex: &Mutex<Box<T>>) -> MutexGuard<'_, Box<T>> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeOfDay;

    /// Keeps whatever it is set to, as an RTC would, without running.
    #[derive(Clone, Default)]
    struct FakeKeeper(Arc<Mutex<Option<UtcTime>>>);

    impl TimeKeeper for FakeKeeper {
        fn read(&mut self) -> Option<UtcTime> {
            *self.0.lock().unwrap()
        }
        fn set(&mut self, time: UtcTime) {
            *self.0.lock().unwrap() = Some(time);
        }
    }

    /// Answers each result in turn.
    struct Scripted(Vec<Result<UtcTime, Unavailable>>);

    impl TimeSource for Scripted {
        fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
            self.0.remove(0)
        }
    }

    #[derive(Clone, Default)]
    struct FakeZones(Arc<Mutex<Option<TimeZone>>>);

    impl TimeZoneSource for FakeZones {
        fn time_zone(&mut self) -> Option<TimeZone> {
            *self.0.lock().unwrap()
        }
    }

    /// 2026-09-26 07:30:15 UTC.
    const MORNING: UtcTime = UtcTime::from_unix_seconds(1_790_407_815);

    fn clock(keeper: &FakeKeeper, answers: Vec<Result<UtcTime, Unavailable>>, zones: &FakeZones) -> Clock {
        Clock::new(Box::new(keeper.clone()), Box::new(Scripted(answers)), Box::new(zones.clone()))
    }

    fn hour(clock: &Clock) -> Option<u8> {
        clock.now().map(|now| now.time_of_day.hour())
    }

    #[test]
    fn nobody_knows_the_time_until_the_keeper_is_read() {
        let keeper = FakeKeeper::default();
        let clock = clock(&keeper, vec![], &FakeZones::default());
        clock.tick(Instant::now());
        assert_eq!(clock.now(), None);
        keeper.clone().set(MORNING);
        assert_eq!(clock.now(), None);
        clock.tick(Instant::now());
        assert_eq!(hour(&clock), Some(9), "central European summer time without a zone chosen");
    }

    #[test]
    fn a_sync_sets_the_keeper_and_shows_at_once() {
        let keeper = FakeKeeper::default();
        let clock = clock(&keeper, vec![Ok(MORNING)], &FakeZones::default());
        assert_eq!(clock.sync_if_due(Instant::now()), Ok(()));
        assert_eq!(keeper.clone().read(), Some(MORNING));
        assert_eq!(clock.now().map(|now| now.time_of_day), TimeOfDay::new(9, 30));
    }

    #[test]
    fn syncs_come_daily_and_retries_sooner() {
        let failed = Unavailable("no Wi-Fi".into());
        let clock = clock(&FakeKeeper::default(), vec![Err(failed.clone()), Ok(MORNING)], &FakeZones::default());
        let start = Instant::now();
        assert!(clock.is_sync_due(start));
        assert_eq!(clock.sync_if_due(start), Err(failed));
        assert!(!clock.is_sync_due(start + RETRY_SYNC_AFTER - Duration::from_secs(1)));
        let retry = start + RETRY_SYNC_AFTER;
        assert_eq!(clock.sync_if_due(retry), Ok(()));
        assert!(!clock.is_sync_due(retry + SYNC_EVERY - Duration::from_secs(1)));
        assert!(clock.is_sync_due(retry + SYNC_EVERY));
    }

    #[test]
    fn a_new_time_zone_shows_within_a_minute() {
        let (keeper, zones) = (FakeKeeper::default(), FakeZones::default());
        keeper.clone().set(MORNING);
        let clock = clock(&keeper, vec![], &zones);
        let start = Instant::now();
        clock.tick(start);
        *zones.0.lock().unwrap() = Some(TimeZone::UTC);
        clock.tick(start + READ_ZONE_EVERY - Duration::from_secs(1));
        assert_eq!(hour(&clock), Some(9));
        clock.tick(start + READ_ZONE_EVERY);
        assert_eq!(hour(&clock), Some(7));
    }
}
