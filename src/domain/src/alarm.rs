//! The alarm clock: a time to wake up on each day of the week, or none. Half an hour
//! before, the lights rise like the sun; then it rings, softly at first, until it is
//! stopped, snoozed, or has rung for a quarter of an hour. Wherever the device is, it
//! comes to the front to be stopped.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::apps::{App, Foreground};
use crate::calendar::Weekday;
use crate::lighting::Level;
use crate::time::{LocalTime, TimeOfDay};

pub const SUNRISE: Duration = Duration::from_secs(30 * 60);
pub const SNOOZE: Duration = Duration::from_secs(9 * 60);
/// Also how late an alarm may still ring: one missed by more, while the device was off
/// or the clock was being set, stays silent.
pub const RING_FOR: Duration = Duration::from_secs(15 * 60);
pub const VOLUME_RISES_OVER: Duration = Duration::from_secs(60);
const FIRST_VOLUME: Volume = Volume::percent(10);

/// Percent of as loud as the ringer rings, 0 to 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Volume(u8);

impl Volume {
    pub const fn percent(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    pub fn as_percent(self) -> u8 {
        self.0
    }
}

/// When to wake up, day by day, and whether to at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlarmSchedule {
    pub enabled: bool,
    /// Monday first; `None` sleeps in.
    times: [Option<TimeOfDay>; 7],
}

impl AlarmSchedule {
    pub fn time_on(&self, day: Weekday) -> Option<TimeOfDay> {
        self.times.get(day.days_since_monday()).copied().flatten()
    }

    pub fn set_time_on(&mut self, day: Weekday, time: Option<TimeOfDay>) {
        if let Some(slot) = self.times.get_mut(day.days_since_monday()) {
            *slot = time;
        }
    }

    /// The first alarm at `from` or after, whether enabled or not.
    pub fn next_from(&self, from: LocalTime) -> Option<LocalTime> {
        let from_seconds = from.seconds_since_epoch();
        (0..=7).find_map(|days| {
            let date = from.date.plus_days(days);
            let time_of_day = self.time_on(date.weekday())?;
            let at = LocalTime { date, time_of_day, second: 0 };
            (at.seconds_since_epoch() >= from_seconds).then_some(at)
        })
    }
}

/// Where the schedule is kept. A store that cannot be read or written leaves the device
/// running on the schedule it has in memory.
pub trait AlarmScheduleStore: Send {
    /// `None` when nothing has been kept yet.
    fn load(&mut self) -> Option<AlarmSchedule>;
    fn save(&mut self, schedule: &AlarmSchedule);
}

pub trait Ringer: Send {
    /// Rings, or goes on ringing, at `volume`.
    fn ring(&mut self, volume: Volume);
    fn silence(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlarmState {
    /// Nothing to do but wait for the next alarm, if any is enabled.
    Waiting { next: Option<LocalTime> },
    Ringing,
    Snoozed { until: LocalTime },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    Ringing { alarm: LocalTime, since: LocalTime },
    Snoozed { alarm: LocalTime, until: LocalTime },
}

struct Inner {
    schedule: AlarmSchedule,
    phase: Phase,
    /// The last alarm that rang, so that it rings once even when the clock goes back.
    rung: Option<LocalTime>,
    now: Option<LocalTime>,
    sounding: bool,
    /// Moves on at every change a screen shows.
    revision: u64,
    store: Box<dyn AlarmScheduleStore>,
    ringer: Box<dyn Ringer>,
    foreground: Foreground,
}

/// Every clone is the same alarm clock.
#[derive(Clone)]
pub struct AlarmClock(Arc<Mutex<Inner>>);

impl AlarmClock {
    /// On the schedule the store kept, or on one with no alarm.
    pub fn new(mut store: Box<dyn AlarmScheduleStore>, ringer: Box<dyn Ringer>, foreground: Foreground) -> Self {
        let schedule = store.load().unwrap_or_default();
        let inner = Inner { schedule, phase: Phase::Waiting, rung: None, now: None, sounding: false, revision: 0, store, ringer, foreground };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub fn schedule(&self) -> AlarmSchedule {
        self.lock().schedule
    }

    pub fn state(&self) -> AlarmState {
        let inner = self.lock();
        match inner.phase {
            Phase::Waiting => AlarmState::Waiting { next: inner.next_alarm() },
            Phase::Ringing { .. } => AlarmState::Ringing,
            Phase::Snoozed { until, .. } => AlarmState::Snoozed { until },
        }
    }

    pub fn revision(&self) -> u64 {
        self.lock().revision
    }

    pub fn switch_on(&self) {
        self.change_schedule(|schedule| schedule.enabled = true);
    }

    /// Also stops it ringing.
    pub fn switch_off(&self) {
        self.change_schedule(|schedule| schedule.enabled = false);
        self.stop();
    }

    pub fn switch_on_or_off(&self) {
        if self.schedule().enabled { self.switch_off() } else { self.switch_on() }
    }

    pub fn set_time_on(&self, day: Weekday, time: Option<TimeOfDay>) {
        self.change_schedule(|schedule| schedule.set_time_on(day, time));
    }

    /// Until the next alarm.
    pub fn stop(&self) {
        let mut inner = self.lock();
        if let Phase::Ringing { alarm, .. } | Phase::Snoozed { alarm, .. } = inner.phase {
            inner.rung = Some(alarm);
            inner.enter(Phase::Waiting);
        }
    }

    /// Rings again [`SNOOZE`] from now.
    pub fn snooze(&self) {
        let mut inner = self.lock();
        if let (Phase::Ringing { alarm, .. }, Some(now)) = (inner.phase, inner.now) {
            let until = LocalTime::from_seconds_since_epoch(now.seconds_since_epoch() + seconds(SNOOZE));
            inner.enter(Phase::Snoozed { alarm, until });
        }
    }

    /// How bright the sunrise is: nothing until [`SUNRISE`] before the alarm, all the
    /// light there is once it rings.
    pub fn sunrise(&self) -> Level {
        let inner = self.lock();
        match (inner.phase, inner.now) {
            (Phase::Ringing { .. } | Phase::Snoozed { .. }, _) => Level::percent(100),
            (Phase::Waiting, Some(now)) => inner.next_alarm().map_or(Level::OFF, |alarm| {
                let ahead = alarm.seconds_since_epoch() - now.seconds_since_epoch();
                percent_of(seconds(SUNRISE) - ahead, seconds(SUNRISE))
            }),
            (Phase::Waiting, None) => Level::OFF,
        }
    }

    /// Moves on to `now`: rings when an alarm comes, stops after [`RING_FOR`].
    pub fn tick(&self, now: LocalTime) {
        let mut inner = self.lock();
        inner.now = Some(now);
        let now_seconds = now.seconds_since_epoch();
        match inner.phase {
            Phase::Waiting => {
                if let Some(alarm) = inner.next_alarm().filter(|alarm| alarm.seconds_since_epoch() <= now_seconds) {
                    inner.enter(Phase::Ringing { alarm, since: now });
                    inner.foreground.bring_to_front(App::Alarm);
                }
            }
            Phase::Ringing { alarm, since } if now_seconds - since.seconds_since_epoch() >= seconds(RING_FOR) => {
                inner.rung = Some(alarm);
                inner.enter(Phase::Waiting);
            }
            Phase::Ringing { .. } => {}
            Phase::Snoozed { alarm, until } if now_seconds >= until.seconds_since_epoch() => {
                inner.enter(Phase::Ringing { alarm, since: now });
                inner.foreground.bring_to_front(App::Alarm);
            }
            Phase::Snoozed { .. } => {}
        }
        inner.sound(now_seconds);
    }

    fn change_schedule(&self, change: impl FnOnce(&mut AlarmSchedule)) {
        let mut inner = self.lock();
        change(&mut inner.schedule);
        let schedule = inner.schedule;
        inner.store.save(&schedule);
        inner.revision = inner.revision.wrapping_add(1);
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Inner {
    /// The next alarm to ring, if the schedule is enabled: one that already came but
    /// less than [`RING_FOR`] ago and has not rung yet counts.
    fn next_alarm(&self) -> Option<LocalTime> {
        let now = self.now?;
        if !self.schedule.enabled {
            return None;
        }
        let late = now.seconds_since_epoch() - seconds(RING_FOR) + 1;
        let after_rung = self.rung.map_or(i64::MIN, |rung| rung.seconds_since_epoch() + 1);
        self.schedule.next_from(LocalTime::from_seconds_since_epoch(late.max(after_rung)))
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Rings louder and louder while ringing, and silences the ringer once otherwise.
    fn sound(&mut self, now_seconds: i64) {
        match self.phase {
            Phase::Ringing { since, .. } => {
                let rising = percent_of(now_seconds - since.seconds_since_epoch(), seconds(VOLUME_RISES_OVER));
                let first = FIRST_VOLUME.as_percent();
                let risen = u16::from(100 - first) * u16::from(rising.as_percent()) / 100;
                self.ringer.ring(Volume::percent(first.saturating_add(u8::try_from(risen).unwrap_or(100))));
                self.sounding = true;
            }
            _ if self.sounding => {
                self.ringer.silence();
                self.sounding = false;
            }
            _ => {}
        }
    }
}

fn seconds(duration: Duration) -> i64 {
    i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
}

/// `part` of `whole`, held to 0 to 100 %.
fn percent_of(part: i64, whole: i64) -> Level {
    let percent = part.clamp(0, whole).saturating_mul(100) / whole.max(1);
    Level::percent(u8::try_from(percent).unwrap_or(100))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Date;

    #[derive(Clone, Default)]
    struct FakeStore(Arc<Mutex<Option<AlarmSchedule>>>);

    impl AlarmScheduleStore for FakeStore {
        fn load(&mut self) -> Option<AlarmSchedule> {
            *self.0.lock().unwrap()
        }
        fn save(&mut self, schedule: &AlarmSchedule) {
            *self.0.lock().unwrap() = Some(*schedule);
        }
    }

    /// The volume it rings at, `None` when silent.
    #[derive(Clone, Default)]
    struct FakeRinger(Arc<Mutex<Option<u8>>>);

    impl Ringer for FakeRinger {
        fn ring(&mut self, volume: Volume) {
            *self.0.lock().unwrap() = Some(volume.as_percent());
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl FakeRinger {
        fn volume(&self) -> Option<u8> {
            *self.0.lock().unwrap()
        }
    }

    /// Saturday 26 September 2026 at `hour:minute:second`, and `days` later.
    fn at(days: i64, hour: u8, minute: u8, second: u8) -> LocalTime {
        let date = Date::new(2026, 9, 26).unwrap().plus_days(days);
        LocalTime { date, time_of_day: TimeOfDay::new(hour, minute).unwrap(), second }
    }

    fn seven_thirty() -> Option<TimeOfDay> {
        TimeOfDay::new(7, 30)
    }

    /// Enabled, at 7:30 on Saturdays and Sundays.
    fn alarm_clock() -> (AlarmClock, FakeRinger, Foreground) {
        let (ringer, foreground) = (FakeRinger::default(), Foreground::new(App::Weather));
        let alarm = AlarmClock::new(Box::new(FakeStore::default()), Box::new(ringer.clone()), foreground.clone());
        alarm.set_time_on(Weekday::Saturday, seven_thirty());
        alarm.set_time_on(Weekday::Sunday, seven_thirty());
        alarm.switch_on();
        (alarm, ringer, foreground)
    }

    #[test]
    fn each_day_has_its_own_time_or_none() {
        let mut schedule = AlarmSchedule::default();
        schedule.set_time_on(Weekday::Monday, TimeOfDay::new(6, 45));
        schedule.set_time_on(Weekday::Wednesday, TimeOfDay::new(8, 0));
        let friday = at(-1, 9, 0, 0);
        let (monday, wednesday) = (2, 4);
        assert_eq!(schedule.next_from(friday), Some(at(monday, 6, 45, 0)));
        assert_eq!(schedule.next_from(at(monday, 6, 45, 0)), Some(at(monday, 6, 45, 0)));
        assert_eq!(schedule.next_from(at(monday, 6, 45, 1)), Some(at(wednesday, 8, 0, 0)));
        assert_eq!(AlarmSchedule::default().next_from(friday), None);
    }

    #[test]
    fn a_week_from_now_is_still_next() {
        let mut schedule = AlarmSchedule::default();
        schedule.set_time_on(Weekday::Saturday, seven_thirty());
        assert_eq!(schedule.next_from(at(0, 7, 30, 1)), Some(at(7, 7, 30, 0)));
    }

    #[test]
    fn it_rings_at_the_time_brings_itself_forward_and_louder_and_louder() {
        let (alarm, ringer, foreground) = alarm_clock();
        alarm.tick(at(0, 7, 29, 59));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(0, 7, 30, 0)) });
        assert_eq!(ringer.volume(), None);
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        assert_eq!(foreground.app(), App::Alarm);
        assert_eq!(ringer.volume(), Some(FIRST_VOLUME.as_percent()));
        alarm.tick(at(0, 7, 30, 30));
        assert_eq!(ringer.volume(), Some(55));
        alarm.tick(at(0, 7, 31, 10));
        assert_eq!(ringer.volume(), Some(100));
    }

    #[test]
    fn stopped_it_waits_for_the_next_day() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.stop();
        alarm.tick(at(0, 7, 30, 1));
        assert_eq!(ringer.volume(), None);
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });
    }

    #[test]
    fn snoozed_it_rings_again_nine_minutes_later_softly_again() {
        let (alarm, ringer, foreground) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.tick(at(0, 7, 32, 0));
        alarm.snooze();
        alarm.tick(at(0, 7, 32, 0));
        assert_eq!(alarm.state(), AlarmState::Snoozed { until: at(0, 7, 41, 0) });
        assert_eq!(ringer.volume(), None);
        foreground.bring_to_front(App::Radar);
        alarm.tick(at(0, 7, 41, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        assert_eq!(ringer.volume(), Some(FIRST_VOLUME.as_percent()));
        assert_eq!(foreground.app(), App::Alarm);
    }

    #[test]
    fn unanswered_it_gives_up_after_a_quarter_of_an_hour() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.tick(at(0, 7, 44, 59));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        alarm.tick(at(0, 7, 45, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });
        assert_eq!(ringer.volume(), None);
    }

    #[test]
    fn an_alarm_missed_by_less_than_a_quarter_of_an_hour_still_rings_and_not_one_missed_by_more() {
        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 44, 59));
        assert_eq!(alarm.state(), AlarmState::Ringing);

        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 45, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });
    }

    #[test]
    fn a_clock_set_back_does_not_ring_the_same_alarm_twice() {
        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.stop();
        alarm.tick(at(0, 7, 20, 0));
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });
    }

    #[test]
    fn a_disabled_alarm_never_rings_and_disabling_stops_it() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.switch_off();
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: None });

        alarm.switch_on();
        alarm.tick(at(0, 7, 31, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        alarm.switch_off();
        alarm.tick(at(0, 7, 31, 1));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: None });
        assert_eq!(ringer.volume(), None);
    }

    #[test]
    fn the_sun_rises_over_the_half_hour_before_and_stays_up_until_stopped() {
        let (alarm, _, _) = alarm_clock();
        assert_eq!(alarm.sunrise(), Level::OFF, "before any tick, the time is unknown");
        alarm.tick(at(0, 6, 59, 59));
        assert_eq!(alarm.sunrise(), Level::OFF);
        alarm.tick(at(0, 7, 15, 0));
        assert_eq!(alarm.sunrise(), Level::percent(50));
        alarm.tick(at(0, 7, 30, 0));
        alarm.snooze();
        assert_eq!(alarm.sunrise(), Level::percent(100));
        alarm.stop();
        assert_eq!(alarm.sunrise(), Level::OFF);
    }

    #[test]
    fn the_schedule_is_kept_on_every_change_and_comes_back() {
        let store = FakeStore::default();
        let foreground = Foreground::new(App::Weather);
        let alarm = AlarmClock::new(Box::new(store.clone()), Box::new(FakeRinger::default()), foreground.clone());
        alarm.set_time_on(Weekday::Tuesday, TimeOfDay::new(6, 0));
        alarm.switch_on();
        let again = AlarmClock::new(Box::new(store), Box::new(FakeRinger::default()), foreground);
        assert_eq!(again.schedule(), alarm.schedule());
        assert!(again.schedule().enabled);
    }
}
