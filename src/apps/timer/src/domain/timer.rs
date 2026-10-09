//! A countdown, in one of three modes: a simple timer that rings at the end; pomodoro
//! rounds of work and breaks, with a chime between them; or a nap that ends with the light,
//! then a ringing. It is timed on the steady clock: if the clock is set during the
//! countdown, the countdown is not shorter or longer.

use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use domain::apps::Foreground;
use domain::lighting::Level;
pub use domain::sound::Volume;

use crate::ID;

pub const SHORT_BREAK: Minutes = Minutes(5);
pub const LONG_BREAK: Minutes = Minutes(15);
/// Work periods in a pomodoro. The long break comes after the last one.
pub const ROUNDS: u8 = 4;
pub const WAKE_LIGHT_RISES_OVER: Duration = Duration::from_secs(3 * 60);
/// Then it stops by itself.
pub const RING_FOR: Duration = Duration::from_secs(10 * 60);
pub const VOLUME_RISES_OVER: Duration = Duration::from_secs(60);
const FIRST_VOLUME: Volume = Volume::percent(10);
const CHIME_VOLUME: Volume = Volume::percent(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Minutes(u16);

impl Minutes {
    pub const fn new(minutes: u16) -> Self {
        Self(minutes)
    }

    pub fn as_minutes(self) -> u16 {
        self.0
    }

    fn duration(self) -> Duration {
        Duration::from_secs(u64::from(self.0) * 60)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Countdown,
    Pomodoro,
    Nap,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Countdown, Self::Pomodoro, Self::Nap];

    /// Goes back to the first after the last.
    pub fn next(self) -> Self {
        match self {
            Self::Countdown => Self::Pomodoro,
            Self::Pomodoro => Self::Nap,
            Self::Nap => Self::Countdown,
        }
    }

    /// For a pomodoro, the length of its work periods.
    pub fn lengths(self) -> RangeInclusive<Minutes> {
        match self {
            Self::Countdown => Minutes(1)..=Minutes(120),
            Self::Pomodoro => Minutes(5)..=Minutes(60),
            Self::Nap => Minutes(5)..=Minutes(90),
        }
    }

    /// One minute at a time up to 10, then 5 at a time up to one hour, then 15 at a time.
    /// Stops at the shortest and at the longest length.
    pub fn turned(self, minutes: Minutes, detents: i32) -> Minutes {
        let step_up_from = |minutes: u16| match minutes {
            0..=9 => 1,
            10..=59 => 5,
            _ => 15,
        };
        let lengths = self.lengths();
        let mut turned = minutes.0;
        for _ in 0..detents.unsigned_abs() {
            turned = if detents > 0 { turned.saturating_add(step_up_from(turned)) } else { turned.saturating_sub(step_up_from(turned.saturating_sub(1))) };
        }
        Minutes(turned).clamp(*lengths.start(), *lengths.end())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerSettings {
    pub mode: Mode,
    pub countdown: Minutes,
    /// The length of its work periods.
    pub pomodoro: Minutes,
    pub nap: Minutes,
}

impl TimerSettings {
    pub fn length_of(&self, mode: Mode) -> Minutes {
        match mode {
            Mode::Countdown => self.countdown,
            Mode::Pomodoro => self.pomodoro,
            Mode::Nap => self.nap,
        }
    }

    fn set_length_of(&mut self, mode: Mode, length: Minutes) {
        let lengths = mode.lengths();
        let length = length.clamp(*lengths.start(), *lengths.end());
        match mode {
            Mode::Countdown => self.countdown = length,
            Mode::Pomodoro => self.pomodoro = length,
            Mode::Nap => self.nap = length,
        }
    }
}

impl Default for TimerSettings {
    fn default() -> Self {
        Self { mode: Mode::default(), countdown: Minutes(10), pomodoro: Minutes(25), nap: Minutes(20) }
    }
}

pub trait TimerSettingsStore: Send {
    fn load(&mut self) -> Option<TimerSettings>;
    fn save(&mut self, settings: &TimerSettings);
}

pub trait Ringer: Send {
    /// Starts ringing, or continues at `volume` if it is ringing.
    fn ring(&mut self, volume: Volume);
    /// A short phrase, which ends by itself.
    fn chime(&mut self, volume: Volume);
    fn silence(&mut self);
}

/// A period of time that is counted down. The rounds of a pomodoro count from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stretch {
    Countdown,
    Work { round: u8 },
    Break { round: u8 },
    LongBreak,
    Nap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StretchEnd {
    /// With a chime.
    Next(Stretch),
    /// With a chime.
    Rest,
    Ring,
}

impl Stretch {
    fn end(self) -> StretchEnd {
        match self {
            Self::Work { round } if round < ROUNDS => StretchEnd::Next(Self::Break { round }),
            Self::Work { .. } => StretchEnd::Next(Self::LongBreak),
            Self::Break { round } => StretchEnd::Next(Self::Work { round: round + 1 }),
            Self::LongBreak => StretchEnd::Rest,
            Self::Countdown | Self::Nap => StretchEnd::Ring,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerState {
    Set,
    Counting { stretch: Stretch, left: Duration },
    Paused { stretch: Stretch, left: Duration },
    /// The period that ended.
    Ringing { after: Stretch },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Set,
    Counting { stretch: Stretch, ends_at: Instant },
    Paused { stretch: Stretch, left: Duration },
    Ringing { after: Stretch, since: Instant },
}

struct Inner {
    settings: TimerSettings,
    phase: Phase,
    now: Option<Instant>,
    ringing: bool,
    revision: u64,
    store: Box<dyn TimerSettingsStore>,
    ringer: Box<dyn Ringer>,
    foreground: Foreground,
}

#[derive(Clone)]
pub struct Timer(Arc<Mutex<Inner>>);

impl Timer {
    pub fn new(mut store: Box<dyn TimerSettingsStore>, ringer: Box<dyn Ringer>, foreground: Foreground) -> Self {
        let settings = store.load().unwrap_or_default();
        let inner = Inner { settings, phase: Phase::Set, now: None, ringing: false, revision: 0, store, ringer, foreground };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub fn settings(&self) -> TimerSettings {
        self.lock().settings
    }

    pub fn state(&self) -> TimerState {
        let inner = self.lock();
        match (inner.phase, inner.now) {
            (Phase::Set, _) => TimerState::Set,
            (Phase::Counting { stretch, ends_at }, now) => {
                TimerState::Counting { stretch, left: now.map_or(Duration::ZERO, |now| ends_at.saturating_duration_since(now)) }
            }
            (Phase::Paused { stretch, left }, _) => TimerState::Paused { stretch, left },
            (Phase::Ringing { after, .. }, _) => TimerState::Ringing { after },
        }
    }

    pub fn revision(&self) -> u64 {
        self.lock().revision
    }

    /// Only when the timer is set: not counting, paused or ringing.
    pub fn choose_next_mode(&self) {
        if self.lock().phase == Phase::Set {
            self.change_settings(|settings| settings.mode = settings.mode.next());
        }
    }

    /// Only when the timer is set: not counting, paused or ringing.
    pub fn turn_length(&self, detents: i32) {
        if self.lock().phase == Phase::Set {
            self.change_settings(|settings| {
                let mode = settings.mode;
                settings.set_length_of(mode, mode.turned(settings.length_of(mode), detents));
            });
        }
    }

    pub fn start(&self) {
        let mut inner = self.lock();
        if inner.phase == Phase::Set {
            let stretch = match inner.settings.mode {
                Mode::Countdown => Stretch::Countdown,
                Mode::Pomodoro => Stretch::Work { round: 1 },
                Mode::Nap => Stretch::Nap,
            };
            inner.count(stretch);
        }
    }

    /// A nap cannot be paused: getting up from a nap means stopping it.
    pub fn pause_or_go_on(&self) {
        let mut inner = self.lock();
        let Some(now) = inner.now else {
            return;
        };
        match inner.phase {
            Phase::Counting { stretch, ends_at } if stretch != Stretch::Nap => {
                inner.enter(Phase::Paused { stretch, left: ends_at.saturating_duration_since(now) });
            }
            Phase::Paused { stretch, left } => inner.enter(Phase::Counting { stretch, ends_at: now + left }),
            _ => {}
        }
    }

    pub fn stop(&self) {
        let mut inner = self.lock();
        if inner.phase != Phase::Set {
            inner.enter(Phase::Set);
            inner.sound();
        }
    }

    /// During a nap, the light rises over its last minutes, and stays on while the timer
    /// rings.
    pub fn wake_light(&self) -> Level {
        let inner = self.lock();
        match (inner.phase, inner.now) {
            (Phase::Counting { stretch: Stretch::Nap, ends_at }, Some(now)) => {
                let left = ends_at.saturating_duration_since(now);
                percent_of(WAKE_LIGHT_RISES_OVER.saturating_sub(left), WAKE_LIGHT_RISES_OVER)
            }
            (Phase::Ringing { after: Stretch::Nap, .. }, _) => Level::percent(100),
            _ => Level::OFF,
        }
    }

    pub fn tick(&self, now: Instant) {
        let mut inner = self.lock();
        inner.now = Some(now);
        match inner.phase {
            Phase::Counting { stretch, ends_at } if now >= ends_at => {
                match stretch.end() {
                    StretchEnd::Next(next) => {
                        inner.ringer.chime(CHIME_VOLUME);
                        inner.count(next);
                    }
                    StretchEnd::Rest => {
                        inner.ringer.chime(CHIME_VOLUME);
                        inner.enter(Phase::Set);
                    }
                    StretchEnd::Ring => inner.enter(Phase::Ringing { after: stretch, since: now }),
                }
                inner.foreground.bring_to_front(ID);
            }
            Phase::Ringing { since, .. } if now.saturating_duration_since(since) >= RING_FOR => inner.enter(Phase::Set),
            _ => {}
        }
        inner.sound();
    }

    fn change_settings(&self, change: impl FnOnce(&mut TimerSettings)) {
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

    fn count(&mut self, stretch: Stretch) {
        if let Some(now) = self.now {
            let length = match stretch {
                Stretch::Countdown => self.settings.countdown,
                Stretch::Work { .. } => self.settings.pomodoro,
                Stretch::Break { .. } => SHORT_BREAK,
                Stretch::LongBreak => LONG_BREAK,
                Stretch::Nap => self.settings.nap,
            };
            self.enter(Phase::Counting { stretch, ends_at: now + length.duration() });
        }
    }

    fn sound(&mut self) {
        match (self.phase, self.now) {
            (Phase::Ringing { since, .. }, Some(now)) => {
                let rising = percent_of(now.saturating_duration_since(since), VOLUME_RISES_OVER).as_percent();
                let first = FIRST_VOLUME.as_percent();
                let risen = u16::from(100 - first) * u16::from(rising) / 100;
                self.ringer.ring(Volume::percent(first.saturating_add(u8::try_from(risen).unwrap_or(100))));
                self.ringing = true;
            }
            _ if self.ringing => {
                self.ringer.silence();
                self.ringing = false;
            }
            _ => {}
        }
    }
}

fn percent_of(part: Duration, whole: Duration) -> Level {
    let percent = part.min(whole).as_millis() * 100 / whole.as_millis().max(1);
    Level::percent(u8::try_from(percent).unwrap_or(100))
}

#[cfg(test)]
mod tests {
    use domain::apps::AppId;

    use super::*;

    const WEATHER: AppId = AppId::new("weather");

    #[derive(Clone, Default)]
    struct FakeTimerSettingsStore(Arc<Mutex<Option<TimerSettings>>>);

    impl TimerSettingsStore for FakeTimerSettingsStore {
        fn load(&mut self) -> Option<TimerSettings> {
            *self.0.lock().unwrap()
        }
        fn save(&mut self, settings: &TimerSettings) {
            *self.0.lock().unwrap() = Some(*settings);
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Heard {
        Ringing(u8),
        Chime,
        Silence,
    }

    #[derive(Clone, Default)]
    struct StubRinger(Arc<Mutex<Vec<Heard>>>);

    impl Ringer for StubRinger {
        fn ring(&mut self, volume: Volume) {
            self.0.lock().unwrap().push(Heard::Ringing(volume.as_percent()));
        }
        fn chime(&mut self, _: Volume) {
            self.0.lock().unwrap().push(Heard::Chime);
        }
        fn silence(&mut self) {
            self.0.lock().unwrap().push(Heard::Silence);
        }
    }

    impl StubRinger {
        fn last(&self) -> Option<Heard> {
            self.0.lock().unwrap().last().cloned()
        }

        fn chimes(&self) -> usize {
            self.0.lock().unwrap().iter().filter(|heard| **heard == Heard::Chime).count()
        }
    }

    struct Bench {
        timer: Timer,
        ringer: StubRinger,
        foreground: Foreground,
        start: Instant,
    }

    impl Bench {
        fn new() -> Self {
            let (ringer, foreground) = (StubRinger::default(), Foreground::new(WEATHER));
            let timer = Timer::new(Box::new(FakeTimerSettingsStore::default()), Box::new(ringer.clone()), foreground.clone());
            let bench = Self { timer, ringer, foreground, start: Instant::now() };
            bench.after(Duration::ZERO);
            bench
        }

        fn in_mode(mode: Mode) -> Self {
            let bench = Self::new();
            while bench.timer.settings().mode != mode {
                bench.timer.choose_next_mode();
            }
            bench
        }

        fn after(&self, elapsed: Duration) {
            self.timer.tick(self.start + elapsed);
        }

        fn after_minutes(&self, minutes: u64) {
            self.after(Duration::from_secs(minutes * 60));
        }

        fn left(&self) -> Duration {
            match self.timer.state() {
                TimerState::Counting { left, .. } | TimerState::Paused { left, .. } => left,
                state => panic!("not counting: {state:?}"),
            }
        }

        fn stretch(&self) -> Stretch {
            match self.timer.state() {
                TimerState::Counting { stretch, .. } | TimerState::Paused { stretch, .. } => stretch,
                state => panic!("not counting: {state:?}"),
            }
        }
    }

    #[test]
    fn the_wheel_sets_minutes_finely_when_short_coarsely_when_long_within_each_mode() {
        let turned = |mode: Mode, minutes, detents| mode.turned(Minutes(minutes), detents).as_minutes();
        assert_eq!(turned(Mode::Countdown, 8, 3), 15);
        assert_eq!(turned(Mode::Countdown, 15, -2), 9);
        assert_eq!(turned(Mode::Countdown, 55, 2), 75);
        assert_eq!(turned(Mode::Countdown, 75, -2), 55);
        assert_eq!(turned(Mode::Countdown, 2, -5), 1);
        assert_eq!(turned(Mode::Countdown, 105, 9), 120);
        assert_eq!(turned(Mode::Pomodoro, 25, 100), 60);
        assert_eq!(turned(Mode::Nap, 20, -100), 5);
    }

    #[test]
    fn the_mode_goes_round_each_with_its_own_length_kept() {
        let bench = Bench::new();
        assert_eq!(bench.timer.settings().mode, Mode::Countdown);
        bench.timer.turn_length(1);
        bench.timer.choose_next_mode();
        assert_eq!(bench.timer.settings().mode, Mode::Pomodoro);
        bench.timer.turn_length(-1);
        bench.timer.choose_next_mode();
        bench.timer.choose_next_mode();
        let settings = bench.timer.settings();
        assert_eq!((settings.mode, settings.countdown, settings.pomodoro, settings.nap), (Mode::Countdown, Minutes(15), Minutes(20), Minutes(20)));
    }

    #[test]
    fn a_countdown_rings_once_over_brings_itself_forward_louder_and_louder_until_stopped() {
        let bench = Bench::new();
        bench.timer.start();
        assert_eq!(bench.timer.state(), TimerState::Counting { stretch: Stretch::Countdown, left: Duration::from_secs(600) });
        bench.after(Duration::from_secs(599));
        assert_eq!(bench.left(), Duration::from_secs(1));
        assert_eq!(bench.ringer.last(), None);
        bench.after_minutes(10);
        assert_eq!(bench.timer.state(), TimerState::Ringing { after: Stretch::Countdown });
        assert_eq!(bench.foreground.app(), ID);
        assert_eq!(bench.ringer.last(), Some(Heard::Ringing(FIRST_VOLUME.as_percent())));
        bench.after(Duration::from_secs(600 + 30));
        assert_eq!(bench.ringer.last(), Some(Heard::Ringing(55)));
        bench.after_minutes(12);
        assert_eq!(bench.ringer.last(), Some(Heard::Ringing(100)));
        bench.timer.stop();
        assert_eq!((bench.timer.state(), bench.ringer.last()), (TimerState::Set, Some(Heard::Silence)));
    }

    #[test]
    fn a_ringing_left_alone_stops_on_its_own() {
        let bench = Bench::new();
        bench.timer.start();
        bench.after_minutes(10);
        bench.after(Duration::from_secs(600) + RING_FOR);
        assert_eq!((bench.timer.state(), bench.ringer.last()), (TimerState::Set, Some(Heard::Silence)));
    }

    #[test]
    fn a_pause_holds_the_time_left_and_going_on_counts_it_down_again() {
        let bench = Bench::new();
        bench.timer.start();
        bench.after_minutes(4);
        bench.timer.pause_or_go_on();
        bench.after_minutes(30);
        assert_eq!(bench.timer.state(), TimerState::Paused { stretch: Stretch::Countdown, left: Duration::from_secs(360) });
        bench.timer.pause_or_go_on();
        bench.after_minutes(35);
        assert_eq!(bench.timer.state(), TimerState::Counting { stretch: Stretch::Countdown, left: Duration::from_secs(60) });
        bench.after_minutes(36);
        assert!(matches!(bench.timer.state(), TimerState::Ringing { .. }));
    }

    #[test]
    fn stopping_a_count_rests_silently_and_the_settings_turn_only_at_rest() {
        let bench = Bench::new();
        bench.timer.start();
        bench.timer.turn_length(3);
        bench.timer.choose_next_mode();
        assert_eq!(bench.timer.settings(), TimerSettings::default());
        bench.timer.stop();
        assert_eq!((bench.timer.state(), bench.ringer.last()), (TimerState::Set, None));
    }

    #[test]
    fn a_pomodoro_chimes_from_work_to_break_four_rounds_then_the_long_break_and_rests() {
        let bench = Bench::in_mode(Mode::Pomodoro);
        bench.timer.start();
        let mut minutes = 0;
        for round in 1..=ROUNDS {
            assert_eq!(bench.stretch(), Stretch::Work { round });
            minutes += 25;
            bench.after_minutes(minutes);
            if round < ROUNDS {
                assert_eq!(bench.stretch(), Stretch::Break { round });
                minutes += 5;
                bench.after_minutes(minutes);
            }
        }
        assert_eq!(bench.timer.state(), TimerState::Counting { stretch: Stretch::LongBreak, left: LONG_BREAK.duration() });
        assert_eq!(bench.ringer.chimes(), 7);
        assert_eq!(bench.foreground.app(), ID);
        bench.after_minutes(minutes + 15);
        assert_eq!((bench.timer.state(), bench.ringer.chimes()), (TimerState::Set, 8));
        assert_eq!(bench.ringer.last(), Some(Heard::Chime), "no ringing after a chime");
    }

    #[test]
    fn a_nap_wakes_with_the_light_rising_then_rings_and_cannot_be_paused() {
        let bench = Bench::in_mode(Mode::Nap);
        bench.timer.start();
        bench.timer.pause_or_go_on();
        assert_eq!(bench.stretch(), Stretch::Nap);
        assert!(matches!(bench.timer.state(), TimerState::Counting { .. }));
        bench.after_minutes(17);
        assert_eq!(bench.timer.wake_light(), Level::OFF);
        bench.after(Duration::from_secs(18 * 60 + 30));
        assert_eq!(bench.timer.wake_light(), Level::percent(50));
        bench.after_minutes(20);
        assert_eq!(bench.timer.state(), TimerState::Ringing { after: Stretch::Nap });
        assert_eq!(bench.timer.wake_light(), Level::percent(100));
        bench.timer.stop();
        assert_eq!(bench.timer.wake_light(), Level::OFF);
    }

    #[test]
    fn a_countdown_wants_no_light() {
        let bench = Bench::new();
        bench.timer.start();
        bench.after(Duration::from_secs(599));
        assert_eq!(bench.timer.wake_light(), Level::OFF);
        bench.after_minutes(10);
        assert_eq!(bench.timer.wake_light(), Level::OFF);
    }

    #[test]
    fn the_settings_are_kept_on_every_change_and_come_back() {
        let store = FakeTimerSettingsStore::default();
        let timer = Timer::new(Box::new(store.clone()), Box::new(StubRinger::default()), Foreground::new(WEATHER));
        timer.choose_next_mode();
        timer.turn_length(2);
        let again = Timer::new(Box::new(store), Box::new(StubRinger::default()), Foreground::new(WEATHER));
        assert_eq!(again.settings(), timer.settings());
        assert_eq!(again.settings().pomodoro, Minutes(35));
    }
}
