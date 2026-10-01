//! A session plays a sound for whole hours, rising at its start and falling at its end,
//! while the lights dim. It starts at the time set when the clock goes past it between two
//! readings, as the alarm rings, or at once when asked. A session's length is kept on the
//! steady clock: a clock set meanwhile neither shortens nor lengthens it.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use domain::apps::Foreground;
use domain::lighting::Level;
use domain::time::{LocalTime, TimeOfDay};
pub use domain::sound::Volume;

use crate::ID;

pub const FADE_IN: Duration = Duration::from_secs(60);
pub const FADE_OUT: Duration = Duration::from_secs(5 * 60);
pub const STOP_FADE: Duration = Duration::from_secs(2);
/// The lights go out over this, or over the session when it is shorter.
pub const DUSK: Duration = Duration::from_secs(30 * 60);
/// How far behind the clock a start time it jumped over may be and still start.
pub const STARTS_WITHIN: Duration = Duration::from_secs(15 * 60);
/// Enough to see by while falling asleep, too little to keep anyone awake.
const DUSK_LIGHT: Level = Level::percent(30);
const PREVIEW_VOLUME: Volume = Volume::percent(30);
const VOLUME_STEP: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionLength(u8);

impl SessionLength {
    pub const SHORTEST: Self = Self(1);
    pub const LONGEST: Self = Self(12);

    pub fn hours(hours: u8) -> Option<Self> {
        (Self::SHORTEST.0..=Self::LONGEST.0).contains(&hours).then_some(Self(hours))
    }

    pub fn as_hours(self) -> u8 {
        self.0
    }

    /// Stops at either end.
    pub fn turned_by(self, detents: i32) -> Self {
        let hours = (i32::from(self.0) + detents).clamp(i32::from(Self::SHORTEST.0), i32::from(Self::LONGEST.0));
        Self(u8::try_from(hours).unwrap_or(self.0))
    }

    fn duration(self) -> Duration {
        Duration::from_secs(u64::from(self.0) * 3600)
    }
}

impl Default for SessionLength {
    fn default() -> Self {
        Self::SHORTEST
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Soundscape {
    /// Played without a file, so there is always one.
    #[default]
    BrownNoise,
    /// One of the app's files, by its name.
    Recorded(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZenSettings {
    /// `None` starts no session on its own.
    pub starts_at: Option<TimeOfDay>,
    pub length: SessionLength,
    pub soundscape: Soundscape,
    pub volume: Volume,
}

impl Default for ZenSettings {
    fn default() -> Self {
        Self { starts_at: None, length: SessionLength::default(), soundscape: Soundscape::default(), volume: Volume::percent(30) }
    }
}

pub trait ZenSettingsStore: Send {
    fn load(&mut self) -> Option<ZenSettings>;
    fn save(&mut self, settings: &ZenSettings);
}

pub trait Player: Send {
    /// The names of the files it can play.
    fn recordings(&self) -> Vec<String>;
    /// Starts `soundscape`, or goes on with it at `volume` when it is the one playing.
    fn play(&mut self, soundscape: &Soundscape, volume: Volume);
    fn silence(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZenState {
    Resting,
    InSession { left: Duration },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Resting,
    /// Resting, the sound playing softly meanwhile.
    Previewing,
    InSession { since: Instant },
    Stopping { since: Instant, from: Volume },
}

struct Inner {
    settings: ZenSettings,
    phase: Phase,
    last_reading: Option<LocalTime>,
    now: Option<Instant>,
    sounding: bool,
    revision: u64,
    store: Box<dyn ZenSettingsStore>,
    player: Box<dyn Player>,
    foreground: Foreground,
}

#[derive(Clone)]
pub struct ZenSessions(Arc<Mutex<Inner>>);

impl ZenSessions {
    pub fn new(mut store: Box<dyn ZenSettingsStore>, player: Box<dyn Player>, foreground: Foreground) -> Self {
        let settings = store.load().unwrap_or_default();
        let inner =
            Inner { settings, phase: Phase::Resting, last_reading: None, now: None, sounding: false, revision: 0, store, player, foreground };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub fn settings(&self) -> ZenSettings {
        self.lock().settings.clone()
    }

    /// Brown noise first, always there.
    pub fn soundscapes(&self) -> Vec<Soundscape> {
        let recordings = self.lock().player.recordings();
        std::iter::once(Soundscape::BrownNoise).chain(recordings.into_iter().map(Soundscape::Recorded)).collect()
    }

    pub fn state(&self) -> ZenState {
        let inner = self.lock();
        match (inner.phase, inner.now) {
            (Phase::InSession { since }, Some(now)) => {
                ZenState::InSession { left: inner.settings.length.duration().saturating_sub(now.saturating_duration_since(since)) }
            }
            _ => ZenState::Resting,
        }
    }

    pub fn revision(&self) -> u64 {
        self.lock().revision
    }

    pub fn set_start(&self, starts_at: Option<TimeOfDay>) {
        self.change_settings(|settings| settings.starts_at = starts_at);
    }

    pub fn set_length(&self, length: SessionLength) {
        self.change_settings(|settings| settings.length = length);
    }

    /// Previewing, the new one plays at once.
    pub fn set_soundscape(&self, soundscape: Soundscape) {
        self.change_settings(|settings| settings.soundscape = soundscape);
        self.lock().sound();
    }

    /// In a session, heard at once.
    pub fn turn_volume(&self, detents: i32) {
        let percent = i32::from(self.settings().volume.as_percent()) + detents * i32::from(VOLUME_STEP);
        let volume = Volume::percent(u8::try_from(percent.clamp(i32::from(VOLUME_STEP), 100)).unwrap_or(100));
        self.change_settings(|settings| settings.volume = volume);
        self.lock().sound();
    }

    /// Plays the sound softly until the preview ends, or a session starts.
    pub fn preview(&self) {
        let mut inner = self.lock();
        if inner.phase == Phase::Resting {
            inner.phase = Phase::Previewing;
            inner.sound();
        }
    }

    pub fn end_preview(&self) {
        let mut inner = self.lock();
        if inner.phase == Phase::Previewing {
            inner.phase = Phase::Resting;
            inner.sound();
        }
    }

    pub fn start(&self) {
        let mut inner = self.lock();
        if let (Phase::Resting | Phase::Previewing, Some(now)) = (inner.phase, inner.now) {
            inner.enter(Phase::InSession { since: now });
            inner.sound();
        }
    }

    /// The sound fades out quickly, the lights go out at once.
    pub fn stop(&self) {
        let mut inner = self.lock();
        if let (Phase::InSession { since }, Some(now)) = (inner.phase, inner.now) {
            let from = inner.session_volume(now.saturating_duration_since(since));
            inner.enter(Phase::Stopping { since: now, from });
            inner.sound();
        }
    }

    pub fn dusk(&self) -> Level {
        let inner = self.lock();
        let (Phase::InSession { since }, Some(now)) = (inner.phase, inner.now) else {
            return Level::OFF;
        };
        let over = DUSK.min(inner.settings.length.duration());
        let left = over.saturating_sub(now.saturating_duration_since(since));
        Level::percent(u8::try_from(u128::from(DUSK_LIGHT.as_percent()) * left.as_millis() / over.as_millis().max(1)).unwrap_or(0))
    }

    pub fn tick(&self, now: Instant, local: Option<LocalTime>) {
        let mut inner = self.lock();
        inner.now = Some(now);
        let previous = std::mem::replace(&mut inner.last_reading, local);
        match inner.phase {
            Phase::Resting | Phase::Previewing => {
                let starts = match (previous, local, inner.settings.starts_at) {
                    (Some(previous), Some(local), Some(starts_at)) => passed(starts_at, previous, local),
                    _ => false,
                };
                if starts {
                    inner.enter(Phase::InSession { since: now });
                    inner.foreground.bring_to_front(ID);
                }
            }
            Phase::InSession { since } if now.saturating_duration_since(since) >= inner.settings.length.duration() => {
                inner.enter(Phase::Resting);
            }
            Phase::Stopping { since, .. } if now.saturating_duration_since(since) >= STOP_FADE => inner.enter(Phase::Resting),
            Phase::InSession { .. } | Phase::Stopping { .. } => {}
        }
        inner.sound();
    }

    fn change_settings(&self, change: impl FnOnce(&mut ZenSettings)) {
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
    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.revision = self.revision.wrapping_add(1);
    }

    /// The volume chosen, risen to over the start and fallen from over the end.
    fn session_volume(&self, played: Duration) -> Volume {
        let left = self.settings.length.duration().saturating_sub(played);
        let share = fraction(played, FADE_IN).min(fraction(left, FADE_OUT));
        scaled(self.settings.volume, share)
    }

    fn sound(&mut self) {
        let volume = match (self.phase, self.now) {
            (Phase::InSession { since }, Some(now)) => Some(self.session_volume(now.saturating_duration_since(since))),
            (Phase::Stopping { since, from }, Some(now)) => {
                Some(scaled(from, 1.0 - fraction(now.saturating_duration_since(since), STOP_FADE)))
            }
            (Phase::Previewing, _) => Some(PREVIEW_VOLUME),
            _ => None,
        };
        match volume {
            Some(volume) => {
                self.player.play(&self.settings.soundscape, volume);
                self.sounding = true;
            }
            None if self.sounding => {
                self.player.silence();
                self.sounding = false;
            }
            None => {}
        }
    }
}

/// Whether the clock went past `time` from `previous` to `now`, by less than [`STARTS_WITHIN`].
fn passed(time: TimeOfDay, previous: LocalTime, now: LocalTime) -> bool {
    let today = LocalTime { date: now.date, time_of_day: time, second: 0 };
    let latest = if today.seconds_since_epoch() <= now.seconds_since_epoch() { today } else { LocalTime { date: now.date.plus_days(-1), ..today } };
    let at = latest.seconds_since_epoch();
    let within = i64::try_from(STARTS_WITHIN.as_secs()).unwrap_or(i64::MAX);
    previous.seconds_since_epoch() < at && now.seconds_since_epoch() - at < within
}

/// `part` of `whole`, from 0 to 1.
fn fraction(part: Duration, whole: Duration) -> f32 {
    (part.as_secs_f32() / whole.as_secs_f32().max(f32::EPSILON)).clamp(0.0, 1.0)
}

fn scaled(volume: Volume, share: f32) -> Volume {
    Volume::percent((f32::from(volume.as_percent()) * share).round() as u8)
}

#[cfg(test)]
mod tests {
    use domain::apps::AppId;
    use domain::calendar::Date;

    use super::*;

    const WEATHER: AppId = AppId::new("weather");

    #[derive(Clone, Default)]
    struct FakeZenSettingsStore(Arc<Mutex<Option<ZenSettings>>>);

    impl ZenSettingsStore for FakeZenSettingsStore {
        fn load(&mut self) -> Option<ZenSettings> {
            self.0.lock().unwrap().clone()
        }
        fn save(&mut self, settings: &ZenSettings) {
            *self.0.lock().unwrap() = Some(settings.clone());
        }
    }

    #[derive(Clone, Default)]
    struct StubPlayer(Arc<Mutex<Option<(Soundscape, u8)>>>);

    impl Player for StubPlayer {
        fn recordings(&self) -> Vec<String> {
            vec!["Rain.wav".into()]
        }
        fn play(&mut self, soundscape: &Soundscape, volume: Volume) {
            *self.0.lock().unwrap() = Some((soundscape.clone(), volume.as_percent()));
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl StubPlayer {
        fn volume(&self) -> Option<u8> {
            self.0.lock().unwrap().as_ref().map(|(_, volume)| *volume)
        }

        fn soundscape(&self) -> Option<Soundscape> {
            self.0.lock().unwrap().as_ref().map(|(soundscape, _)| soundscape.clone())
        }
    }

    fn rain() -> Soundscape {
        Soundscape::Recorded("Rain.wav".into())
    }

    /// Saturday 26 September 2026 at `hour:minute:second`, and `days` later.
    fn at(days: i64, hour: u8, minute: u8, second: u8) -> LocalTime {
        let date = Date::new(2026, 9, 26).unwrap().plus_days(days);
        LocalTime { date, time_of_day: TimeOfDay::new(hour, minute).unwrap(), second }
    }

    /// The steady clock and the wall clock, moving on together.
    struct Bench {
        zen: ZenSessions,
        player: StubPlayer,
        foreground: Foreground,
        start: Instant,
        start_local: LocalTime,
    }

    impl Bench {
        /// At 22:00, a session of 2 hours set for 22:30, at 40 %.
        fn new() -> Self {
            let (player, foreground) = (StubPlayer::default(), Foreground::new(WEATHER));
            let zen = ZenSessions::new(Box::new(FakeZenSettingsStore::default()), Box::new(player.clone()), foreground.clone());
            zen.set_start(TimeOfDay::new(22, 30));
            zen.set_length(SessionLength::hours(2).unwrap());
            zen.turn_volume(2);
            let bench = Self { zen, player, foreground, start: Instant::now(), start_local: at(0, 22, 0, 0) };
            bench.after(Duration::ZERO);
            bench
        }

        fn after(&self, elapsed: Duration) {
            let seconds = self.start_local.seconds_since_epoch() + i64::try_from(elapsed.as_secs()).unwrap();
            self.zen.tick(self.start + elapsed, Some(LocalTime::from_seconds_since_epoch(seconds)));
        }

        fn after_minutes(&self, minutes: u64) {
            self.after(Duration::from_secs(minutes * 60));
        }
    }

    #[test]
    fn a_length_is_whole_hours_from_one_to_twelve() {
        assert_eq!(SessionLength::hours(0), None);
        assert_eq!(SessionLength::hours(13), None);
        assert_eq!(SessionLength::LONGEST.turned_by(3), SessionLength::LONGEST);
        assert_eq!(SessionLength::SHORTEST.turned_by(-1), SessionLength::SHORTEST);
        assert_eq!(SessionLength::SHORTEST.turned_by(4).as_hours(), 5);
    }

    #[test]
    fn a_session_starts_at_its_time_brings_itself_forward_and_rises() {
        let bench = Bench::new();
        bench.after_minutes(29);
        assert_eq!(bench.zen.state(), ZenState::Resting);
        assert_eq!(bench.player.volume(), None);
        bench.after_minutes(30);
        assert_eq!(bench.zen.state(), ZenState::InSession { left: Duration::from_secs(2 * 3600) });
        assert_eq!(bench.foreground.app(), ID);
        assert_eq!(bench.player.volume(), Some(0));
        bench.after(Duration::from_secs(30 * 60 + 30));
        assert_eq!(bench.player.volume(), Some(20));
        bench.after_minutes(31);
        assert_eq!(bench.player.volume(), Some(40));
    }

    #[test]
    fn a_session_falls_over_its_last_minutes_and_ends_after_its_hours() {
        let bench = Bench::new();
        bench.after_minutes(30);
        bench.after(Duration::from_secs(30 * 60 + 2 * 3600 - 150));
        assert_eq!(bench.player.volume(), Some(20));
        assert!(matches!(bench.zen.state(), ZenState::InSession { .. }));
        bench.after_minutes(30 + 120);
        assert_eq!(bench.zen.state(), ZenState::Resting);
        assert_eq!(bench.player.volume(), None);
    }

    #[test]
    fn the_same_time_the_next_day_starts_another() {
        let bench = Bench::new();
        bench.after_minutes(30);
        bench.after_minutes(150);
        bench.after_minutes(24 * 60 + 29);
        assert_eq!(bench.zen.state(), ZenState::Resting);
        bench.after_minutes(24 * 60 + 30);
        assert!(matches!(bench.zen.state(), ZenState::InSession { .. }));
    }

    #[test]
    fn no_time_set_starts_nothing_and_a_clock_set_far_over_it_neither() {
        let bench = Bench::new();
        bench.after_minutes(29);
        bench.after_minutes(46);
        assert_eq!(bench.zen.state(), ZenState::Resting, "a quarter of an hour late");
        bench.zen.set_start(None);
        bench.after_minutes(24 * 60 + 29);
        bench.after_minutes(24 * 60 + 30);
        assert_eq!(bench.zen.state(), ZenState::Resting);
    }

    #[test]
    fn a_time_passed_while_the_device_was_off_waits_for_the_next_day() {
        let player = StubPlayer::default();
        let zen = ZenSessions::new(Box::new(FakeZenSettingsStore::default()), Box::new(player.clone()), Foreground::new(WEATHER));
        zen.set_start(TimeOfDay::new(22, 30));
        zen.tick(Instant::now(), Some(at(0, 22, 31, 0)));
        assert_eq!(zen.state(), ZenState::Resting);
        assert_eq!(player.volume(), None);
    }

    #[test]
    fn started_at_once_it_lasts_its_hours_and_stopped_it_fades_out_quickly() {
        let bench = Bench::new();
        bench.zen.start();
        assert_eq!(bench.zen.state(), ZenState::InSession { left: Duration::from_secs(2 * 3600) });
        assert_eq!(bench.foreground.app(), WEATHER, "started from the app, it is already in front");
        bench.after_minutes(10);
        assert_eq!(bench.player.volume(), Some(40));
        bench.zen.stop();
        assert_eq!(bench.zen.state(), ZenState::Resting);
        assert_eq!(bench.player.volume(), Some(40));
        bench.after(Duration::from_secs(10 * 60 + 1));
        assert_eq!(bench.player.volume(), Some(20));
        bench.after(Duration::from_secs(10 * 60 + 2));
        assert_eq!(bench.player.volume(), None);
    }

    #[test]
    fn its_time_passing_during_a_session_starts_no_other() {
        let bench = Bench::new();
        bench.zen.start();
        bench.after_minutes(30);
        assert_eq!(bench.zen.state(), ZenState::InSession { left: Duration::from_secs(90 * 60) });
    }

    #[test]
    fn the_wheel_turns_the_volume_in_steps_and_keeps_it() {
        let bench = Bench::new();
        bench.zen.start();
        bench.after_minutes(2);
        bench.zen.turn_volume(-1);
        assert_eq!(bench.player.volume(), Some(35));
        bench.zen.turn_volume(-20);
        assert_eq!(bench.zen.settings().volume, Volume::percent(VOLUME_STEP), "never silent");
        bench.zen.turn_volume(30);
        assert_eq!(bench.player.volume(), Some(100));
    }

    #[test]
    fn the_lights_dim_over_half_an_hour_or_the_session_and_go_out_when_stopped() {
        let bench = Bench::new();
        assert_eq!(bench.zen.dusk(), Level::OFF);
        bench.zen.start();
        assert_eq!(bench.zen.dusk(), DUSK_LIGHT);
        bench.after_minutes(15);
        assert_eq!(bench.zen.dusk(), Level::percent(15));
        bench.after_minutes(30);
        assert_eq!(bench.zen.dusk(), Level::OFF);
        bench.zen.stop();
        bench.zen.start();
        bench.zen.stop();
        assert_eq!(bench.zen.dusk(), Level::OFF);
    }

    #[test]
    fn a_preview_plays_softly_each_sound_chosen_until_it_ends() {
        let bench = Bench::new();
        let revision = bench.zen.revision();
        bench.zen.preview();
        assert_eq!(bench.player.soundscape(), Some(Soundscape::BrownNoise));
        assert_eq!(bench.player.volume(), Some(PREVIEW_VOLUME.as_percent()));
        bench.zen.set_soundscape(rain());
        assert_eq!(bench.player.soundscape(), Some(rain()));
        assert_ne!(bench.zen.revision(), revision);
        bench.zen.end_preview();
        assert_eq!(bench.player.volume(), None);
    }

    #[test]
    fn a_session_takes_over_a_preview() {
        let bench = Bench::new();
        bench.after_minutes(29);
        bench.zen.preview();
        bench.after_minutes(30);
        assert!(matches!(bench.zen.state(), ZenState::InSession { .. }));
        bench.zen.end_preview();
        bench.after_minutes(32);
        assert_eq!(bench.player.volume(), Some(40), "ending the preview does not end the session");
    }

    #[test]
    fn the_sounds_are_brown_noise_then_the_files() {
        assert_eq!(Bench::new().zen.soundscapes(), [Soundscape::BrownNoise, rain()]);
    }

    #[test]
    fn the_settings_are_kept_on_every_change_and_come_back() {
        let store = FakeZenSettingsStore::default();
        let zen = ZenSessions::new(Box::new(store.clone()), Box::new(StubPlayer::default()), Foreground::new(WEATHER));
        zen.set_start(TimeOfDay::new(21, 45));
        zen.set_length(SessionLength::hours(8).unwrap());
        zen.set_soundscape(rain());
        zen.turn_volume(-1);
        let again = ZenSessions::new(Box::new(store), Box::new(StubPlayer::default()), Foreground::new(WEATHER));
        assert_eq!(again.settings(), zen.settings());
        assert_eq!(again.settings().volume, Volume::percent(25));
    }
}
