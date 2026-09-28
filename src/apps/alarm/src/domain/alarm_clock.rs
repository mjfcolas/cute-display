//! An alarm rings when the clock goes past its time between two readings: a clock set
//! forward over it by less than [`RING_FOR`] still rings it, and a time set, or the alarm
//! switched on, after it passed does not. Nothing is caught up at start, and a clock set
//! back over an alarm that rang rings it again.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use domain::apps::Foreground;
use domain::calendar::Weekday;
use domain::lighting::Level;
use domain::time::{LocalTime, TimeOfDay};

use crate::ID;

pub const SUNRISE: Duration = Duration::from_secs(30 * 60);
pub const SNOOZE: Duration = Duration::from_secs(9 * 60);
/// Also how far behind the clock an alarm it jumped over may be and still ring.
pub const RING_FOR: Duration = Duration::from_secs(15 * 60);
pub const VOLUME_RISES_OVER: Duration = Duration::from_secs(60);
const FIRST_VOLUME: Volume = Volume::percent(10);
const PREVIEW_VOLUME: Volume = Volume::percent(30);

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

    /// Whether enabled or not.
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Ringtone {
    /// Played without a file, so there is always one.
    #[default]
    Chime,
    /// One of the alarm's files, by its name.
    Recorded(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlarmSettings {
    pub schedule: AlarmSchedule,
    pub ringtone: Ringtone,
}

pub trait AlarmSettingsStore: Send {
    fn load(&mut self) -> Option<AlarmSettings>;
    fn save(&mut self, settings: &AlarmSettings);
}

pub trait Ringer: Send {
    /// The names of the files it can ring.
    fn recordings(&self) -> Vec<String>;
    /// Starts `ringtone`, or goes on with it at `volume` when it is the one ringing.
    fn ring(&mut self, ringtone: &Ringtone, volume: Volume);
    fn silence(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlarmState {
    Waiting { next: Option<LocalTime> },
    Ringing,
    Snoozed { until: LocalTime },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    /// Waiting, the ringtone playing softly meanwhile.
    Previewing,
    Ringing { since: LocalTime },
    Snoozed { until: LocalTime },
}

struct Inner {
    settings: AlarmSettings,
    phase: Phase,
    last_reading: Option<LocalTime>,
    sounding: bool,
    revision: u64,
    store: Box<dyn AlarmSettingsStore>,
    ringer: Box<dyn Ringer>,
    foreground: Foreground,
}

#[derive(Clone)]
pub struct AlarmClock(Arc<Mutex<Inner>>);

impl AlarmClock {
    pub fn new(mut store: Box<dyn AlarmSettingsStore>, ringer: Box<dyn Ringer>, foreground: Foreground) -> Self {
        let settings = store.load().unwrap_or_default();
        let (phase, last_reading) = (Phase::Waiting, None);
        let inner = Inner { settings, phase, last_reading, sounding: false, revision: 0, store, ringer, foreground };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub fn schedule(&self) -> AlarmSchedule {
        self.lock().settings.schedule
    }

    pub fn ringtone(&self) -> Ringtone {
        self.lock().settings.ringtone.clone()
    }

    /// The chime first, always there.
    pub fn ringtones(&self) -> Vec<Ringtone> {
        let recordings = self.lock().ringer.recordings();
        std::iter::once(Ringtone::Chime).chain(recordings.into_iter().map(Ringtone::Recorded)).collect()
    }

    /// Previewing, the new one plays at once.
    pub fn set_ringtone(&self, ringtone: Ringtone) {
        self.change_settings(|settings| settings.ringtone = ringtone);
        self.lock().sound();
    }

    /// Plays the ringtone softly until the preview ends, or the alarm rings.
    pub fn preview(&self) {
        let mut inner = self.lock();
        if inner.phase == Phase::Waiting {
            inner.phase = Phase::Previewing;
            inner.sound();
        }
    }

    pub fn end_preview(&self) {
        let mut inner = self.lock();
        if inner.phase == Phase::Previewing {
            inner.phase = Phase::Waiting;
            inner.sound();
        }
    }

    pub fn state(&self) -> AlarmState {
        let inner = self.lock();
        match inner.phase {
            Phase::Waiting | Phase::Previewing => AlarmState::Waiting { next: inner.next_alarm() },
            Phase::Ringing { .. } => AlarmState::Ringing,
            Phase::Snoozed { until } => AlarmState::Snoozed { until },
        }
    }

    pub fn revision(&self) -> u64 {
        self.lock().revision
    }

    pub fn switch_on(&self) {
        self.change_settings(|settings| settings.schedule.enabled = true);
    }

    pub fn switch_off(&self) {
        self.change_settings(|settings| settings.schedule.enabled = false);
        self.stop();
    }

    pub fn switch_on_or_off(&self) {
        if self.schedule().enabled { self.switch_off() } else { self.switch_on() }
    }

    pub fn set_time_on(&self, day: Weekday, time: Option<TimeOfDay>) {
        self.change_settings(|settings| settings.schedule.set_time_on(day, time));
    }

    pub fn stop(&self) {
        let mut inner = self.lock();
        if let Phase::Ringing { .. } | Phase::Snoozed { .. } = inner.phase {
            inner.enter(Phase::Waiting);
        }
    }

    pub fn snooze(&self) {
        let mut inner = self.lock();
        if let (Phase::Ringing { .. }, Some(now)) = (inner.phase, inner.last_reading) {
            let until = LocalTime::from_seconds_since_epoch(now.seconds_since_epoch() + seconds(SNOOZE));
            inner.enter(Phase::Snoozed { until });
        }
    }

    pub fn sunrise(&self) -> Level {
        let inner = self.lock();
        match (inner.phase, inner.last_reading) {
            (Phase::Ringing { .. } | Phase::Snoozed { .. }, _) => Level::percent(100),
            (Phase::Waiting | Phase::Previewing, Some(now)) => inner.next_alarm().map_or(Level::OFF, |alarm| {
                let ahead = alarm.seconds_since_epoch() - now.seconds_since_epoch();
                percent_of(seconds(SUNRISE) - ahead, seconds(SUNRISE))
            }),
            (Phase::Waiting | Phase::Previewing, None) => Level::OFF,
        }
    }

    pub fn tick(&self, now: LocalTime) {
        let mut inner = self.lock();
        let previous = inner.last_reading.replace(now);
        let now_seconds = now.seconds_since_epoch();
        match inner.phase {
            Phase::Waiting | Phase::Previewing => {
                let enabled = inner.settings.schedule.enabled;
                let passed = previous
                    .and_then(|previous| inner.settings.schedule.next_from(one_second_after(previous)))
                    .is_some_and(|alarm| (0..seconds(RING_FOR)).contains(&(now_seconds - alarm.seconds_since_epoch())));
                if enabled && passed {
                    inner.enter(Phase::Ringing { since: now });
                    inner.foreground.bring_to_front(ID);
                }
            }
            Phase::Ringing { since } if now_seconds - since.seconds_since_epoch() >= seconds(RING_FOR) => {
                inner.enter(Phase::Waiting);
            }
            Phase::Ringing { .. } => {}
            Phase::Snoozed { until } if now_seconds >= until.seconds_since_epoch() => {
                inner.enter(Phase::Ringing { since: now });
                inner.foreground.bring_to_front(ID);
            }
            Phase::Snoozed { .. } => {}
        }
        inner.sound();
    }

    fn change_settings(&self, change: impl FnOnce(&mut AlarmSettings)) {
        let mut inner = self.lock();
        change(&mut inner.settings);
        let Inner { store, settings, .. } = &mut *inner;
        store.save(settings);
        inner.revision = inner.revision.wrapping_add(1);
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Inner {
    fn next_alarm(&self) -> Option<LocalTime> {
        let now = self.last_reading?;
        if !self.settings.schedule.enabled {
            return None;
        }
        self.settings.schedule.next_from(one_second_after(now))
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.revision = self.revision.wrapping_add(1);
    }

    fn sound(&mut self) {
        let volume = match (self.phase, self.last_reading) {
            (Phase::Ringing { since }, Some(now)) => {
                let rising = percent_of(now.seconds_since_epoch() - since.seconds_since_epoch(), seconds(VOLUME_RISES_OVER));
                let first = FIRST_VOLUME.as_percent();
                let risen = u16::from(100 - first) * u16::from(rising.as_percent()) / 100;
                Some(Volume::percent(first.saturating_add(u8::try_from(risen).unwrap_or(100))))
            }
            (Phase::Previewing, _) => Some(PREVIEW_VOLUME),
            _ => None,
        };
        match volume {
            Some(volume) => {
                self.ringer.ring(&self.settings.ringtone, volume);
                self.sounding = true;
            }
            None if self.sounding => {
                self.ringer.silence();
                self.sounding = false;
            }
            None => {}
        }
    }
}

fn one_second_after(time: LocalTime) -> LocalTime {
    LocalTime::from_seconds_since_epoch(time.seconds_since_epoch() + 1)
}

fn seconds(duration: Duration) -> i64 {
    i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
}

fn percent_of(part: i64, whole: i64) -> Level {
    let percent = part.clamp(0, whole).saturating_mul(100) / whole.max(1);
    Level::percent(u8::try_from(percent).unwrap_or(100))
}

#[cfg(test)]
mod tests {
    use domain::apps::AppId;
    use domain::calendar::Date;

    use super::*;

    const WEATHER: AppId = AppId::new("weather");
    const RADAR: AppId = AppId::new("radar");

    #[derive(Clone, Default)]
    struct FakeStore(Arc<Mutex<Option<AlarmSettings>>>);

    impl AlarmSettingsStore for FakeStore {
        fn load(&mut self) -> Option<AlarmSettings> {
            self.0.lock().unwrap().clone()
        }
        fn save(&mut self, settings: &AlarmSettings) {
            *self.0.lock().unwrap() = Some(settings.clone());
        }
    }

    #[derive(Clone, Default)]
    struct FakeRinger(Arc<Mutex<Option<(Ringtone, u8)>>>);

    impl Ringer for FakeRinger {
        fn recordings(&self) -> Vec<String> {
            vec!["Zen.mp3".into()]
        }
        fn ring(&mut self, ringtone: &Ringtone, volume: Volume) {
            *self.0.lock().unwrap() = Some((ringtone.clone(), volume.as_percent()));
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl FakeRinger {
        fn volume(&self) -> Option<u8> {
            self.0.lock().unwrap().as_ref().map(|(_, volume)| *volume)
        }

        fn ringtone(&self) -> Option<Ringtone> {
            self.0.lock().unwrap().as_ref().map(|(ringtone, _)| ringtone.clone())
        }
    }

    fn zen() -> Ringtone {
        Ringtone::Recorded("Zen.mp3".into())
    }

    /// Saturday 26 September 2026 at `hour:minute:second`, and `days` later.
    fn at(days: i64, hour: u8, minute: u8, second: u8) -> LocalTime {
        let date = Date::new(2026, 9, 26).unwrap().plus_days(days);
        LocalTime { date, time_of_day: TimeOfDay::new(hour, minute).unwrap(), second }
    }

    fn seven_thirty() -> Option<TimeOfDay> {
        TimeOfDay::new(7, 30)
    }

    fn alarm_clock() -> (AlarmClock, FakeRinger, Foreground) {
        let (ringer, foreground) = (FakeRinger::default(), Foreground::new(WEATHER));
        let alarm = AlarmClock::new(Box::new(FakeStore::default()), Box::new(ringer.clone()), foreground.clone());
        alarm.set_time_on(Weekday::Saturday, seven_thirty());
        alarm.set_time_on(Weekday::Sunday, seven_thirty());
        alarm.switch_on();
        alarm.tick(at(0, 6, 0, 0));
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
        assert_eq!(foreground.app(), ID);
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
        foreground.bring_to_front(RADAR);
        alarm.tick(at(0, 7, 41, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        assert_eq!(ringer.volume(), Some(FIRST_VOLUME.as_percent()));
        assert_eq!(foreground.app(), ID);
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
    fn a_clock_set_forward_over_an_alarm_rings_it() {
        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 29, 59));
        alarm.tick(at(0, 7, 30, 2));
        assert_eq!(alarm.state(), AlarmState::Ringing);
    }

    #[test]
    fn a_clock_set_forward_far_over_an_alarm_does_not_ring_it() {
        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 29, 59));
        alarm.tick(at(0, 7, 45, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });
    }

    #[test]
    fn a_clock_set_back_over_an_alarm_that_rang_rings_it_again() {
        let (alarm, _, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.stop();
        alarm.tick(at(0, 7, 20, 0));
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
    }

    #[test]
    fn an_alarm_passed_while_the_device_was_off_stays_silent() {
        let alarm = AlarmClock::new(Box::new(FakeStore::default()), Box::new(FakeRinger::default()), Foreground::new(WEATHER));
        alarm.set_time_on(Weekday::Saturday, seven_thirty());
        alarm.switch_on();
        assert_eq!(alarm.sunrise(), Level::OFF, "before any reading, the time is unknown");
        alarm.tick(at(0, 7, 30, 1));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(7, 7, 30, 0)) });
    }

    #[test]
    fn a_disabled_alarm_never_rings_and_disabling_stops_it() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.switch_off();
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: None });

        alarm.switch_on();
        alarm.tick(at(1, 7, 30, 0));
        assert_eq!(alarm.state(), AlarmState::Ringing);
        alarm.switch_off();
        alarm.tick(at(1, 7, 30, 1));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: None });
        assert_eq!(ringer.volume(), None);
    }

    #[test]
    fn a_time_set_or_switched_on_just_past_waits_for_next_week() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.stop();
        alarm.tick(at(0, 7, 45, 0));
        alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(7, 40));
        alarm.tick(at(0, 7, 45, 1));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(1, 7, 30, 0)) });

        alarm.switch_off();
        alarm.tick(at(1, 7, 29, 0));
        alarm.tick(at(1, 7, 35, 0));
        alarm.switch_on();
        alarm.tick(at(1, 7, 35, 1));
        assert_eq!(alarm.state(), AlarmState::Waiting { next: Some(at(7, 7, 40, 0)) });
        assert_eq!(ringer.volume(), None);
    }

    #[test]
    fn the_sun_rises_over_the_half_hour_before_and_stays_up_until_stopped() {
        let (alarm, _, _) = alarm_clock();
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
    fn it_rings_the_ringtone_chosen() {
        let (alarm, ringer, _) = alarm_clock();
        assert_eq!(alarm.ringtone(), Ringtone::Chime);
        assert_eq!(alarm.ringtones(), [Ringtone::Chime, zen()]);
        alarm.set_ringtone(zen());
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(ringer.ringtone(), Some(zen()));
    }

    #[test]
    fn a_preview_plays_softly_each_ringtone_chosen_until_it_ends() {
        let (alarm, ringer, _) = alarm_clock();
        let revision = alarm.revision();
        alarm.preview();
        assert_eq!(ringer.ringtone(), Some(Ringtone::Chime));
        assert_eq!(ringer.volume(), Some(PREVIEW_VOLUME.as_percent()));
        alarm.set_ringtone(zen());
        assert_eq!(ringer.ringtone(), Some(zen()));
        assert_ne!(alarm.revision(), revision);
        alarm.tick(at(0, 6, 0, 1));
        assert_eq!(ringer.volume(), Some(PREVIEW_VOLUME.as_percent()));
        alarm.end_preview();
        assert_eq!(ringer.volume(), None);
    }

    #[test]
    fn the_alarm_takes_over_a_preview_and_it_does_not_come_back() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.tick(at(0, 7, 29, 59));
        alarm.preview();
        alarm.tick(at(0, 7, 30, 0));
        assert_eq!(ringer.volume(), Some(FIRST_VOLUME.as_percent()));
        alarm.stop();
        alarm.tick(at(0, 7, 30, 1));
        assert_eq!(ringer.volume(), None);
        alarm.preview();
        alarm.tick(at(0, 7, 30, 2));
        assert_eq!(ringer.volume(), Some(PREVIEW_VOLUME.as_percent()), "waiting again, a preview may start");
    }

    #[test]
    fn ringing_there_is_no_preview() {
        let (alarm, ringer, _) = alarm_clock();
        alarm.tick(at(0, 7, 30, 0));
        alarm.preview();
        alarm.tick(at(0, 7, 30, 30));
        assert_eq!(ringer.volume(), Some(55));
    }

    #[test]
    fn the_settings_are_kept_on_every_change_and_come_back() {
        let store = FakeStore::default();
        let foreground = Foreground::new(WEATHER);
        let alarm = AlarmClock::new(Box::new(store.clone()), Box::new(FakeRinger::default()), foreground.clone());
        alarm.set_time_on(Weekday::Tuesday, TimeOfDay::new(6, 0));
        alarm.switch_on();
        alarm.set_ringtone(zen());
        let again = AlarmClock::new(Box::new(store), Box::new(FakeRinger::default()), foreground);
        assert_eq!(again.schedule(), alarm.schedule());
        assert!(again.schedule().enabled);
        assert_eq!(again.ringtone(), zen());
    }
}
