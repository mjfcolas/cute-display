mod description;
mod drawing;
mod ui_state;

use std::hash::{DefaultHasher, Hash, Hasher};

use ui::controls::{Button, Input};
use ui::mark::Mark;
use ui::Screen;

pub use ui_state::{BreathePage, RhythmRow, Session};

use crate::domain::breathing::{Breathing, BreathingState, Fullness, Rhythm, Step};

pub struct BreatheScreen {
    breathing: Breathing,
}

impl BreatheScreen {
    pub fn new(breathing: Breathing) -> Self {
        Self { breathing }
    }

    fn page(&self) -> BreathePage {
        let settings = self.breathing.settings();
        let rhythms = Rhythm::ALL.map(|rhythm| RhythmRow {
            name: rhythm_name(rhythm),
            steps: steps(rhythm),
            mark: if rhythm == settings.rhythm { Mark::Chosen } else { Mark::Plain },
        });
        let (session, hint) = match self.breathing.state() {
            BreathingState::Resting => (Session::Resting, "wheel: rhythm   yellow: start"),
            BreathingState::Breathing { since, .. } => {
                (Session::Breathing { minutes: since.as_secs() / 60, volume: settings.volume }, "wheel: volume   yellow: stop")
            }
        };
        BreathePage { breath: one_breath(settings.rhythm), rhythms, session, hint }
    }
}

impl Screen for BreatheScreen {
    type UiState = BreathePage;

    fn ui_state(&self) -> BreathePage {
        self.page()
    }

    /// Once a minute: the lights follow each breath, not the screen.
    fn version(&self) -> u64 {
        let minutes = match self.breathing.state() {
            BreathingState::Breathing { since, .. } => Some(since.as_secs() / 60),
            BreathingState::Resting => None,
        };
        let mut hasher = DefaultHasher::new();
        (self.breathing.revision(), minutes).hash(&mut hasher);
        hasher.finish()
    }

    fn on_input(&mut self, input: Input) {
        match (self.breathing.state(), input) {
            (BreathingState::Resting, Input::Turn(detents)) => self.breathing.turn_rhythm(detents),
            (BreathingState::Resting, Input::Press(Button::Yellow)) => self.breathing.start(),
            (BreathingState::Breathing { .. }, Input::Turn(detents)) => self.breathing.turn_volume(detents),
            (BreathingState::Breathing { .. }, Input::Press(Button::Yellow)) => self.breathing.stop(),
            (_, Input::Press(Button::Long) | Input::HoldYellowAndLong) => {}
        }
    }
}

/// Enough for a smooth curve across the picture.
const BREATH_POINTS: u32 = 64;

fn one_breath(rhythm: Rhythm) -> Vec<Fullness> {
    (0..=BREATH_POINTS).map(|n| rhythm.fullness_at(rhythm.cycle() * n / BREATH_POINTS)).collect()
}

fn rhythm_name(rhythm: Rhythm) -> &'static str {
    match rhythm {
        Rhythm::Coherence => "Coherence",
        Rhythm::FourSevenEight => "4-7-8",
        Rhythm::Square => "Square",
    }
}

fn steps(rhythm: Rhythm) -> String {
    let named: Vec<String> = rhythm
        .steps()
        .into_iter()
        .map(|(step, seconds)| {
            let name = match step {
                Step::BreatheIn => "in",
                Step::HoldIn | Step::HoldOut => "hold",
                Step::BreatheOut => "out",
            };
            format!("{name} {seconds}")
        })
        .collect();
    format!("{} s", named.join(", "))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use domain::sound::Volume;

    use super::*;
    use crate::domain::breathing::{Breath, BreathingSettings, BreathingSettingsStore};

    struct StubBreathingSettingsStore;
    impl BreathingSettingsStore for StubBreathingSettingsStore {
        fn load(&mut self) -> Option<BreathingSettings> {
            None
        }
        fn save(&mut self, _: &BreathingSettings) {}
    }

    #[derive(Clone, Default)]
    struct StubBreath(Arc<Mutex<bool>>);
    impl Breath for StubBreath {
        fn sound_at(&mut self, _: Volume) {
            *self.0.lock().unwrap() = true;
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = false;
        }
    }

    struct Bench {
        screen: BreatheScreen,
        breathing: Breathing,
        breath: StubBreath,
        start: Instant,
    }

    impl Bench {
        fn new() -> Self {
            let breath = StubBreath::default();
            let breathing = Breathing::new(Box::new(StubBreathingSettingsStore), Box::new(breath.clone()));
            let bench = Self { screen: BreatheScreen::new(breathing.clone()), breathing, breath, start: Instant::now() };
            bench.after(Duration::ZERO);
            bench
        }

        fn after(&self, elapsed: Duration) {
            self.breathing.tick(self.start + elapsed);
        }

        fn input(&mut self, input: Input) {
            Screen::on_input(&mut self.screen, input);
        }

        fn chosen(&self) -> &'static str {
            let chosen: Vec<&'static str> = self.screen.ui_state().rhythms.iter().filter(|row| row.mark == Mark::Chosen).map(|row| row.name).collect();
            let [name] = chosen[..] else { panic!("rhythms chosen: {chosen:?}") };
            name
        }
    }

    #[test]
    fn resting_the_wheel_chooses_the_rhythm_and_yellow_starts() {
        let mut bench = Bench::new();
        assert_eq!((bench.chosen(), bench.screen.ui_state().hint), ("Coherence", "wheel: rhythm   yellow: start"));
        bench.input(Input::Turn(2));
        assert_eq!(bench.chosen(), "Square");
        bench.input(Input::Press(Button::Long));
        bench.input(Input::Press(Button::Yellow));
        assert!(matches!(bench.breathing.state(), BreathingState::Breathing { .. }));
        assert!(*bench.breath.0.lock().unwrap());
    }

    #[test]
    fn breathing_the_wheel_turns_the_volume_and_yellow_stops() {
        let mut bench = Bench::new();
        bench.input(Input::Press(Button::Yellow));
        bench.input(Input::Turn(1));
        assert_eq!((bench.chosen(), bench.screen.ui_state().session), ("Coherence", Session::Breathing { minutes: 0, volume: Volume::percent(35) }));
        assert_eq!(bench.screen.ui_state().hint, "wheel: volume   yellow: stop");
        bench.input(Input::Press(Button::Long));
        bench.input(Input::HoldYellowAndLong);
        assert!(matches!(bench.breathing.state(), BreathingState::Breathing { .. }));
        bench.input(Input::Press(Button::Yellow));
        assert_eq!((bench.breathing.state(), bench.screen.ui_state().session), (BreathingState::Resting, Session::Resting));
        assert!(!*bench.breath.0.lock().unwrap());
    }

    #[test]
    fn the_time_breathing_is_said_and_redrawn_by_the_minute_not_by_the_breath() {
        let mut bench = Bench::new();
        bench.input(Input::Press(Button::Yellow));
        let version = Screen::version(&bench.screen);
        assert_eq!(bench.screen.ui_state().session.status(), "Breathing");
        bench.after(Duration::from_secs(59));
        assert_eq!(Screen::version(&bench.screen), version);
        bench.after(Duration::from_secs(3 * 60 + 5));
        assert_ne!(Screen::version(&bench.screen), version);
        assert_eq!(bench.screen.ui_state().session.status(), "Breathing for 3 min");
    }

    #[test]
    fn the_breath_drawn_is_the_chosen_rhythms_full_while_held() {
        let mut bench = Bench::new();
        let coherence = bench.screen.ui_state().breath;
        let halfway = (BREATH_POINTS / 2) as usize;
        assert_eq!((coherence.first(), coherence.get(halfway), coherence.last()), (Some(&Fullness::EMPTY), Some(&Fullness::FULL), Some(&Fullness::EMPTY)));
        bench.input(Input::Turn(1));
        let four_seven_eight = bench.screen.ui_state().breath;
        let full = four_seven_eight.iter().filter(|&&fullness| fullness == Fullness::FULL).count() as u64;
        let (cycle, held) = (Rhythm::FourSevenEight.cycle().as_secs(), 7);
        assert!(full * cycle >= held * u64::from(BREATH_POINTS), "held {held} of {cycle} seconds, full {full} of {BREATH_POINTS} points");
    }

    #[test]
    fn each_rhythm_says_its_steps_in_seconds() {
        let said: Vec<String> = Rhythm::ALL.into_iter().map(steps).collect();
        assert_eq!(said, ["in 5, out 5 s", "in 4, hold 7, out 8 s", "in 4, hold 4, out 4, hold 4 s"]);
    }
}
