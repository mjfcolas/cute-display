//! Breathing to a rhythm until stopped: the lights rise as one breathes in, stay up while
//! one holds, fall as one breathes out; a breath of sound rises and falls with each
//! breath in and each breath out, silent while one holds. Kept on the steady clock.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use domain::lighting::Level;
pub use domain::sound::Volume;

/// Enough to follow with the eyes half closed, too little to wake anyone up.
pub const BREATH_LIGHT: Level = Level::percent(30);
const VOLUME_STEP: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    BreatheIn,
    HoldIn,
    BreatheOut,
    HoldOut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Rhythm {
    /// Six breaths a minute.
    #[default]
    Coherence,
    /// To fall asleep to.
    FourSevenEight,
    Square,
}

impl Rhythm {
    pub const ALL: [Self; 3] = [Self::Coherence, Self::FourSevenEight, Self::Square];

    /// With their seconds.
    pub fn steps(self) -> Vec<(Step, u8)> {
        let [breathe_in, hold_in, breathe_out, hold_out] = match self {
            Self::Coherence => [5, 0, 5, 0],
            Self::FourSevenEight => [4, 7, 8, 0],
            Self::Square => [4, 4, 4, 4],
        };
        [(Step::BreatheIn, breathe_in), (Step::HoldIn, hold_in), (Step::BreatheOut, breathe_out), (Step::HoldOut, hold_out)]
            .into_iter()
            .filter(|&(_, seconds)| seconds > 0)
            .collect()
    }

    pub fn turned_round_by(self, detents: i32) -> Self {
        let at = Self::ALL.iter().position(|rhythm| *rhythm == self).unwrap_or(0) as i64;
        let turned = (at + i64::from(detents)).rem_euclid(Self::ALL.len() as i64);
        Self::ALL.get(turned as usize).copied().unwrap_or(self)
    }

    /// Round and round, however long since the start.
    pub fn fullness_at(self, since_start: Duration) -> Fullness {
        let share = match self.step_at(since_start) {
            (Step::BreatheIn, into) => into.eased(),
            (Step::HoldIn, _) => 1.0,
            (Step::BreatheOut, into) => 1.0 - into.eased(),
            (Step::HoldOut, _) => 0.0,
        };
        Fullness((share.clamp(0.0, 1.0) * 100.0).round() as u8)
    }

    pub fn cycle(self) -> Duration {
        Duration::from_secs(self.steps().iter().map(|&(_, seconds)| u64::from(seconds)).sum())
    }

    fn step_at(self, elapsed: Duration) -> (Step, Progress) {
        let cycle = self.cycle().as_millis().max(1);
        let mut into = elapsed.as_millis() % cycle;
        for (step, seconds) in self.steps() {
            let length = u128::from(seconds) * 1000;
            if into < length {
                return (step, Progress::of(into, length));
            }
            into -= length;
        }
        (Step::BreatheIn, Progress::of(0, 1))
    }
}

/// How full of air one is, from empty to full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fullness(u8);

impl Fullness {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(100);

    pub fn as_percent(self) -> u8 {
        self.0
    }
}

/// How far into a step, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Progress(f32);

impl Progress {
    fn of(part: u128, whole: u128) -> Self {
        Self((part as f32 / whole.max(1) as f32).clamp(0.0, 1.0))
    }

    /// Slow at both ends, as a breath is.
    fn eased(self) -> f32 {
        (1.0 - (self.0 * PI).cos()) / 2.0
    }

    /// Up and down again over the step.
    fn swelling(self) -> f32 {
        (self.0 * PI).sin()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreathingSettings {
    pub rhythm: Rhythm,
    pub volume: Volume,
}

impl Default for BreathingSettings {
    fn default() -> Self {
        Self { rhythm: Rhythm::default(), volume: Volume::percent(30) }
    }
}

pub trait BreathingSettingsStore: Send {
    fn load(&mut self) -> Option<BreathingSettings>;
    fn save(&mut self, settings: &BreathingSettings);
}

pub trait Breath: Send {
    /// Starts the breath of sound, or goes on with it at `volume`.
    fn sound_at(&mut self, volume: Volume);
    fn silence(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreathingState {
    Resting,
    Breathing { step: Step, since: Duration },
}

struct Inner {
    settings: BreathingSettings,
    started: Option<Instant>,
    now: Option<Instant>,
    revision: u64,
    store: Box<dyn BreathingSettingsStore>,
    breath: Box<dyn Breath>,
}

#[derive(Clone)]
pub struct Breathing(Arc<Mutex<Inner>>);

impl Breathing {
    pub fn new(mut store: Box<dyn BreathingSettingsStore>, breath: Box<dyn Breath>) -> Self {
        let settings = store.load().unwrap_or_default();
        Self(Arc::new(Mutex::new(Inner { settings, started: None, now: None, revision: 0, store, breath })))
    }

    pub fn settings(&self) -> BreathingSettings {
        self.lock().settings
    }

    pub fn state(&self) -> BreathingState {
        let inner = self.lock();
        match inner.elapsed() {
            Some(since) => BreathingState::Breathing { step: inner.settings.rhythm.step_at(since).0, since },
            None => BreathingState::Resting,
        }
    }

    pub fn revision(&self) -> u64 {
        self.lock().revision
    }

    /// Resting only.
    pub fn turn_rhythm(&self, detents: i32) {
        if self.lock().started.is_none() {
            self.change_settings(|settings| settings.rhythm = settings.rhythm.turned_round_by(detents));
        }
    }

    /// Never silent; heard at once.
    pub fn turn_volume(&self, detents: i32) {
        let percent = i32::from(self.settings().volume.as_percent()) + detents * i32::from(VOLUME_STEP);
        let volume = Volume::percent(u8::try_from(percent.clamp(i32::from(VOLUME_STEP), 100)).unwrap_or(100));
        self.change_settings(|settings| settings.volume = volume);
        self.lock().sound();
    }

    pub fn start(&self) {
        let mut inner = self.lock();
        if let (None, Some(now)) = (inner.started, inner.now) {
            inner.started = Some(now);
            inner.revision = inner.revision.wrapping_add(1);
            inner.sound();
        }
    }

    pub fn stop(&self) {
        let mut inner = self.lock();
        if inner.started.take().is_some() {
            inner.revision = inner.revision.wrapping_add(1);
            inner.breath.silence();
        }
    }

    pub fn light(&self) -> Level {
        let inner = self.lock();
        let Some(elapsed) = inner.elapsed() else {
            return Level::OFF;
        };
        let fullness = inner.settings.rhythm.fullness_at(elapsed);
        Level::percent(u8::try_from(u16::from(BREATH_LIGHT.as_percent()) * u16::from(fullness.as_percent()) / 100).unwrap_or(0))
    }

    pub fn tick(&self, now: Instant) {
        let mut inner = self.lock();
        inner.now = Some(now);
        inner.sound();
    }

    fn change_settings(&self, change: impl FnOnce(&mut BreathingSettings)) {
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
    fn elapsed(&self) -> Option<Duration> {
        Some(self.now?.saturating_duration_since(self.started?))
    }

    /// Rising and falling over each breath, so that it sounds like one.
    fn sound(&mut self) {
        let Some(elapsed) = self.elapsed() else {
            return;
        };
        let share = match self.settings.rhythm.step_at(elapsed) {
            (Step::BreatheIn | Step::BreatheOut, into) => into.swelling(),
            (Step::HoldIn | Step::HoldOut, _) => 0.0,
        };
        self.breath.sound_at(Volume::percent((f32::from(self.settings.volume.as_percent()) * share).round() as u8));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct FakeBreathingSettingsStore(Arc<Mutex<Option<BreathingSettings>>>);

    impl BreathingSettingsStore for FakeBreathingSettingsStore {
        fn load(&mut self) -> Option<BreathingSettings> {
            *self.0.lock().unwrap()
        }
        fn save(&mut self, settings: &BreathingSettings) {
            *self.0.lock().unwrap() = Some(*settings);
        }
    }

    #[derive(Clone, Default)]
    struct StubBreath(Arc<Mutex<Option<u8>>>);

    impl Breath for StubBreath {
        fn sound_at(&mut self, volume: Volume) {
            *self.0.lock().unwrap() = Some(volume.as_percent());
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl StubBreath {
        fn volume(&self) -> Option<u8> {
            *self.0.lock().unwrap()
        }
    }

    struct Bench {
        breathing: Breathing,
        breath: StubBreath,
        start: Instant,
    }

    impl Bench {
        fn new() -> Self {
            let breath = StubBreath::default();
            let breathing = Breathing::new(Box::new(FakeBreathingSettingsStore::default()), Box::new(breath.clone()));
            let bench = Self { breathing, breath, start: Instant::now() };
            bench.after_millis(0);
            bench
        }

        fn breathing(rhythm: Rhythm) -> Self {
            let bench = Self::new();
            while bench.breathing.settings().rhythm != rhythm {
                bench.breathing.turn_rhythm(1);
            }
            bench.breathing.start();
            bench
        }

        fn after_millis(&self, millis: u64) {
            self.breathing.tick(self.start + Duration::from_millis(millis));
        }

        fn step(&self) -> Step {
            match self.breathing.state() {
                BreathingState::Breathing { step, .. } => step,
                BreathingState::Resting => panic!("resting"),
            }
        }
    }

    #[test]
    fn each_rhythm_goes_round_its_steps_in_their_seconds() {
        let bench = Bench::breathing(Rhythm::FourSevenEight);
        let steps: Vec<Step> = [0, 3_900, 4_000, 10_900, 11_000, 18_900, 19_000].into_iter().map(|millis| {
            bench.after_millis(millis);
            bench.step()
        }).collect();
        use Step::*;
        assert_eq!(steps, [BreatheIn, BreatheIn, HoldIn, HoldIn, BreatheOut, BreatheOut, BreatheIn]);
        let square = Bench::breathing(Rhythm::Square);
        square.after_millis(12_500);
        assert_eq!(square.step(), HoldOut);
    }

    #[test]
    fn the_lights_rise_breathing_in_stay_up_holding_and_fall_breathing_out() {
        let bench = Bench::breathing(Rhythm::Square);
        let levels: Vec<u8> = [0, 2_000, 4_000, 6_000, 8_000, 10_000, 13_000].into_iter().map(|millis| {
            bench.after_millis(millis);
            bench.breathing.light().as_percent()
        }).collect();
        assert_eq!(levels, [0, 15, 30, 30, 30, 15, 0]);
    }

    #[test]
    fn a_breath_of_sound_rises_and_falls_with_each_breath_silent_while_holding() {
        let bench = Bench::breathing(Rhythm::FourSevenEight);
        let volumes: Vec<Option<u8>> = [0, 2_000, 6_000, 15_000].into_iter().map(|millis| {
            bench.after_millis(millis);
            bench.breath.volume()
        }).collect();
        assert_eq!(volumes, [Some(0), Some(30), Some(0), Some(30)]);
    }

    #[test]
    fn it_goes_on_until_stopped_then_all_is_dark_and_silent() {
        let bench = Bench::breathing(Rhythm::Coherence);
        bench.after_millis(3 * 3600 * 1000 + 2_500);
        assert_eq!(bench.breathing.state(), BreathingState::Breathing { step: Step::BreatheIn, since: Duration::from_millis(3 * 3600 * 1000 + 2_500) });
        bench.breathing.stop();
        assert_eq!((bench.breathing.state(), bench.breathing.light(), bench.breath.volume()), (BreathingState::Resting, Level::OFF, None));
        bench.after_millis(3 * 3600 * 1000 + 5_000);
        assert_eq!(bench.breath.volume(), None, "no sound resting");
    }

    #[test]
    fn the_rhythm_turns_round_only_resting() {
        let bench = Bench::new();
        bench.breathing.turn_rhythm(-1);
        assert_eq!(bench.breathing.settings().rhythm, Rhythm::Square);
        bench.breathing.start();
        bench.breathing.turn_rhythm(1);
        assert_eq!(bench.breathing.settings().rhythm, Rhythm::Square);
    }

    #[test]
    fn the_wheel_turns_the_volume_in_steps_never_silent_heard_at_once() {
        let bench = Bench::breathing(Rhythm::Coherence);
        bench.after_millis(2_500);
        bench.breathing.turn_volume(2);
        assert_eq!(bench.breath.volume(), Some(40));
        bench.breathing.turn_volume(-20);
        assert_eq!(bench.breathing.settings().volume, Volume::percent(VOLUME_STEP));
    }

    #[test]
    fn the_settings_are_kept_on_every_change_and_come_back() {
        let store = FakeBreathingSettingsStore::default();
        let breathing = Breathing::new(Box::new(store.clone()), Box::new(StubBreath::default()));
        breathing.turn_rhythm(1);
        breathing.turn_volume(-1);
        let again = Breathing::new(Box::new(store), Box::new(StubBreath::default()));
        assert_eq!(again.settings(), BreathingSettings { rhythm: Rhythm::FourSevenEight, volume: Volume::percent(25) });
    }
}
