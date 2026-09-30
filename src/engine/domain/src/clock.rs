use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::fetch::Unavailable;
use crate::shared::Shared;
use crate::time::{LocalTime, UtcTime};
use crate::time_zone::TimeZone;

pub const SYNC_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
pub const RETRY_SYNC_AFTER: Duration = Duration::from_secs(10 * 60);
pub const READ_ZONE_EVERY: Duration = Duration::from_secs(60);

pub trait TimeKeeper: Send {
    fn read(&mut self) -> Option<UtcTime>;
    fn set(&mut self, time: UtcTime);
}

pub trait TimeSource: Send {
    fn fetch(&mut self) -> Result<UtcTime, Unavailable>;
}

pub trait TimeZoneSource: Send {
    fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable>;
}

#[derive(Clone, Copy)]
enum SyncAttempt {
    Succeeded(Instant),
    Failed(Instant),
}

#[derive(Clone, Copy)]
struct State {
    time: Option<UtcTime>,
    ticked_at: Option<Instant>,
    zone: TimeZone,
    zone_read: Option<Instant>,
    last_sync: Option<SyncAttempt>,
}

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
            state: Shared::new(State { time: None, ticked_at: None, zone: TimeZone::default(), zone_read: None, last_sync: None }),
            keeper: Arc::new(Mutex::new(keeper)),
            source: Arc::new(Mutex::new(source)),
            zones: Arc::new(Mutex::new(zones)),
        }
    }

    pub fn now(&self) -> Option<LocalTime> {
        let state = self.state.get();
        state.time.map(|time| state.zone.local(time))
    }

    pub fn ticked_at(&self) -> Option<Instant> {
        self.state.get().ticked_at
    }

    pub fn tick(&self, now: Instant) {
        let time = lock(&self.keeper).read();
        let zone_read = self.state.get().zone_read;
        let zone = zone_read
            .is_none_or(|read| now.saturating_duration_since(read) >= READ_ZONE_EVERY)
            .then(|| lock(&self.zones).time_zone());
        self.state.update(|state| {
            state.time = time;
            state.ticked_at = Some(now);
            if let Some(reading) = zone {
                // An unreadable zone keeps the one the clock had, rather than jumping hours
                // to the default and back.
                if let Ok(chosen) = reading {
                    state.zone = chosen.unwrap_or_default();
                }
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

    #[derive(Clone, Default)]
    struct FakeTimeKeeper(Arc<Mutex<Option<UtcTime>>>);

    impl TimeKeeper for FakeTimeKeeper {
        fn read(&mut self) -> Option<UtcTime> {
            *self.0.lock().unwrap()
        }
        fn set(&mut self, time: UtcTime) {
            *self.0.lock().unwrap() = Some(time);
        }
    }

    struct StubTimeSource(Vec<Result<UtcTime, Unavailable>>);

    impl TimeSource for StubTimeSource {
        fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
            self.0.remove(0)
        }
    }

    #[derive(Clone)]
    struct StubTimeZoneSource(Arc<Mutex<Result<Option<TimeZone>, Unavailable>>>);

    impl Default for StubTimeZoneSource {
        fn default() -> Self {
            Self(Arc::new(Mutex::new(Ok(None))))
        }
    }

    impl TimeZoneSource for StubTimeZoneSource {
        fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
            self.0.lock().unwrap().clone()
        }
    }

    /// 2026-09-26 07:30:15 UTC.
    const MORNING: UtcTime = UtcTime::from_unix_seconds(1_790_407_815);

    fn clock(keeper: &FakeTimeKeeper, answers: Vec<Result<UtcTime, Unavailable>>, zones: &StubTimeZoneSource) -> Clock {
        Clock::new(Box::new(keeper.clone()), Box::new(StubTimeSource(answers)), Box::new(zones.clone()))
    }

    fn hour(clock: &Clock) -> Option<u8> {
        clock.now().map(|now| now.time_of_day.hour())
    }

    #[test]
    fn it_knows_when_it_last_ticked_and_nothing_before_it_did() {
        let clock = clock(&FakeTimeKeeper::default(), vec![], &StubTimeZoneSource::default());
        assert_eq!(clock.ticked_at(), None);
        let at = Instant::now() + Duration::from_secs(90);
        clock.tick(at);
        assert_eq!(clock.ticked_at(), Some(at));
    }

    #[test]
    fn nobody_knows_the_time_until_the_keeper_is_read() {
        let keeper = FakeTimeKeeper::default();
        let clock = clock(&keeper, vec![], &StubTimeZoneSource::default());
        clock.tick(Instant::now());
        assert_eq!(clock.now(), None);
        keeper.clone().set(MORNING);
        assert_eq!(clock.now(), None);
        clock.tick(Instant::now());
        assert_eq!(hour(&clock), Some(9), "central European summer time without a zone chosen");
    }

    #[test]
    fn a_sync_sets_the_keeper_and_shows_at_once() {
        let keeper = FakeTimeKeeper::default();
        let clock = clock(&keeper, vec![Ok(MORNING)], &StubTimeZoneSource::default());
        assert_eq!(clock.sync_if_due(Instant::now()), Ok(()));
        assert_eq!(keeper.clone().read(), Some(MORNING));
        assert_eq!(clock.now().map(|now| now.time_of_day), TimeOfDay::new(9, 30));
    }

    #[test]
    fn syncs_come_daily_and_retries_sooner() {
        let failed = Unavailable("no Wi-Fi".into());
        let clock = clock(&FakeTimeKeeper::default(), vec![Err(failed.clone()), Ok(MORNING)], &StubTimeZoneSource::default());
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
        let (keeper, zones) = (FakeTimeKeeper::default(), StubTimeZoneSource::default());
        keeper.clone().set(MORNING);
        let clock = clock(&keeper, vec![], &zones);
        let start = Instant::now();
        clock.tick(start);
        *zones.0.lock().unwrap() = Ok(Some(TimeZone::UTC));
        clock.tick(start + READ_ZONE_EVERY - Duration::from_secs(1));
        assert_eq!(hour(&clock), Some(9));
        clock.tick(start + READ_ZONE_EVERY);
        assert_eq!(hour(&clock), Some(7));
    }

    #[test]
    fn a_zone_that_cannot_be_read_leaves_the_one_in_use_and_none_chosen_is_the_default() {
        let (keeper, zones) = (FakeTimeKeeper::default(), StubTimeZoneSource::default());
        keeper.clone().set(MORNING);
        *zones.0.lock().unwrap() = Ok(Some(TimeZone::UTC));
        let clock = clock(&keeper, vec![], &zones);
        let start = Instant::now();
        clock.tick(start);
        *zones.0.lock().unwrap() = Err(Unavailable("the SD card did not answer".into()));
        clock.tick(start + READ_ZONE_EVERY);
        assert_eq!(hour(&clock), Some(7));
        *zones.0.lock().unwrap() = Ok(None);
        clock.tick(start + READ_ZONE_EVERY * 2);
        assert_eq!(hour(&clock), Some(9));
    }
}
