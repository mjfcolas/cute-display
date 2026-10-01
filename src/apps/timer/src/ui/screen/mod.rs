mod description;
mod drawing;
mod ui_state;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use ui::controls::{Button, Input};
use ui::mark::Mark;
use ui::Screen;

pub use ui_state::{Count, ModeTab, TimerPage};

use crate::domain::timer::{Mode, Stretch, Timer, TimerState, ROUNDS, SHORT_BREAK};

pub struct TimerScreen {
    timer: Timer,
}

impl TimerScreen {
    pub fn new(timer: Timer) -> Self {
        Self { timer }
    }

    fn page(&self) -> TimerPage {
        let settings = self.timer.settings();
        let modes = Mode::ALL.map(|mode| ModeTab { name: mode_name(mode), mark: if mode == settings.mode { Mark::Chosen } else { Mark::Plain } });
        let (count, status, hint) = match self.timer.state() {
            TimerState::Set => (
                Count::Minutes(settings.length_of(settings.mode).as_minutes()),
                ready(settings.mode),
                "wheel: minutes   yellow: start   long: mode",
            ),
            TimerState::Counting { stretch: Stretch::Nap, left } => (count_of(left), counting(Stretch::Nap), "yellow: stop"),
            TimerState::Counting { stretch, left } => (count_of(left), counting(stretch), "yellow: pause   long: stop"),
            TimerState::Paused { stretch, left } => (count_of(left), format!("Paused: {}", counting(stretch)), "yellow: go on   long: stop"),
            TimerState::Ringing { after: Stretch::Nap } => (Count::Seconds(0), "Time to wake up".into(), "yellow: stop"),
            TimerState::Ringing { .. } => (Count::Seconds(0), "Time is up".into(), "yellow: stop"),
        };
        TimerPage { modes, count, status, hint }
    }

    /// Asleep, only yellow does anything, so that nothing half awake changes a setting.
    fn on_timer_input(&self, input: Input) {
        match (self.timer.state(), input) {
            (TimerState::Set, Input::Turn(detents)) => self.timer.turn_length(detents),
            (TimerState::Set, Input::Press(Button::Yellow)) => self.timer.start(),
            (TimerState::Set, Input::Press(Button::Long)) => self.timer.choose_next_mode(),
            (TimerState::Counting { stretch: Stretch::Nap, .. } | TimerState::Ringing { .. }, Input::Press(Button::Yellow)) => self.timer.stop(),
            (TimerState::Counting { stretch: Stretch::Nap, .. }, _) => {}
            (TimerState::Ringing { .. }, Input::Press(Button::Long)) => self.timer.stop(),
            (TimerState::Counting { .. } | TimerState::Paused { .. }, Input::Press(Button::Yellow)) => self.timer.pause_or_go_on(),
            (TimerState::Counting { .. } | TimerState::Paused { .. }, Input::Press(Button::Long)) => self.timer.stop(),
            (_, Input::Turn(_) | Input::HoldYellowAndLong) => {}
        }
    }
}

impl Screen for TimerScreen {
    type UiState = TimerPage;

    fn ui_state(&self) -> TimerPage {
        self.page()
    }

    fn version(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        (self.timer.revision(), self.page().count).hash(&mut hasher);
        hasher.finish()
    }

    fn on_input(&mut self, input: Input) {
        self.on_timer_input(input);
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Countdown => "Timer",
        Mode::Pomodoro => "Pomodoro",
        Mode::Nap => "Nap",
    }
}

fn ready(mode: Mode) -> String {
    match mode {
        Mode::Countdown => "Rings once it is over".into(),
        Mode::Pomodoro => format!("Work, then {} min of rest; {ROUNDS} rounds", SHORT_BREAK.as_minutes()),
        Mode::Nap => "Lights out, then the light rises".into(),
    }
}

fn counting(stretch: Stretch) -> String {
    match stretch {
        Stretch::Countdown => "Counting down".into(),
        Stretch::Work { round } => format!("Work, round {round} of {ROUNDS}"),
        Stretch::Break { round } => format!("Break, round {round} of {ROUNDS}"),
        Stretch::LongBreak => "Long break".into(),
        Stretch::Nap => "Napping".into(),
    }
}

/// A minute or a second begun counts whole.
fn count_of(left: Duration) -> Count {
    let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
    match seconds.div_ceil(60) {
        0 | 1 => Count::Seconds(u8::try_from(seconds).unwrap_or(60)),
        minutes => Count::Minutes(u16::try_from(minutes).unwrap_or(u16::MAX)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use domain::apps::Foreground;

    use super::*;
    use crate::domain::timer::{Ringer, TimerSettings, TimerSettingsStore, Volume};

    struct StubTimerSettingsStore;
    impl TimerSettingsStore for StubTimerSettingsStore {
        fn load(&mut self) -> Option<TimerSettings> {
            None
        }
        fn save(&mut self, _: &TimerSettings) {}
    }

    #[derive(Clone, Default)]
    struct StubRinger(Arc<Mutex<bool>>);
    impl Ringer for StubRinger {
        fn ring(&mut self, _: Volume) {
            *self.0.lock().unwrap() = true;
        }
        fn chime(&mut self, _: Volume) {}
        fn silence(&mut self) {
            *self.0.lock().unwrap() = false;
        }
    }

    struct Bench {
        screen: TimerScreen,
        timer: Timer,
        ringer: StubRinger,
        start: Instant,
    }

    impl Bench {
        fn new() -> Self {
            let ringer = StubRinger::default();
            let timer = Timer::new(Box::new(StubTimerSettingsStore), Box::new(ringer.clone()), Foreground::new(crate::ID));
            let bench = Self { screen: TimerScreen::new(timer.clone()), timer, ringer, start: Instant::now() };
            bench.after(Duration::ZERO);
            bench
        }

        fn after(&self, elapsed: Duration) {
            self.timer.tick(self.start + elapsed);
        }

        fn input(&mut self, input: Input) {
            Screen::on_input(&mut self.screen, input);
        }

        fn press(&mut self, button: Button) {
            self.input(Input::Press(button));
        }

        fn chosen_mode(&self) -> &'static str {
            let page = self.screen.ui_state();
            let chosen: Vec<&'static str> = page.modes.iter().filter(|tab| tab.mark == Mark::Chosen).map(|tab| tab.name).collect();
            let [mode] = chosen[..] else { panic!("modes chosen: {chosen:?}") };
            mode
        }
    }

    #[test]
    fn set_the_wheel_turns_the_minutes_and_the_long_button_the_mode() {
        let mut bench = Bench::new();
        assert_eq!((bench.chosen_mode(), bench.screen.ui_state().count), ("Timer", Count::Minutes(10)));
        bench.input(Input::Turn(-2));
        assert_eq!(bench.screen.ui_state().count, Count::Minutes(8));
        bench.press(Button::Long);
        assert_eq!((bench.chosen_mode(), bench.screen.ui_state().count), ("Pomodoro", Count::Minutes(25)));
        assert_eq!(bench.screen.ui_state().status, "Work, then 5 min of rest; 4 rounds");
        bench.press(Button::Long);
        bench.press(Button::Long);
        assert_eq!((bench.chosen_mode(), bench.screen.ui_state().count), ("Timer", Count::Minutes(8)));
    }

    #[test]
    fn yellow_starts_pauses_and_goes_on_and_long_stops() {
        let mut bench = Bench::new();
        bench.press(Button::Yellow);
        assert_eq!(bench.screen.ui_state().hint, "yellow: pause   long: stop");
        bench.input(Input::Turn(5));
        bench.press(Button::Yellow);
        assert_eq!(bench.screen.ui_state().status, "Paused: Counting down");
        assert_eq!(bench.screen.ui_state().hint, "yellow: go on   long: stop");
        bench.press(Button::Yellow);
        assert_eq!(bench.screen.ui_state().count, Count::Minutes(10), "the wheel changed nothing");
        bench.press(Button::Long);
        assert_eq!(bench.timer.state(), TimerState::Set);
    }

    #[test]
    fn the_count_goes_by_the_minute_then_by_the_second_over_the_last() {
        let mut bench = Bench::new();
        bench.press(Button::Yellow);
        let version = Screen::version(&bench.screen);
        bench.after(Duration::from_secs(30));
        assert_eq!(Screen::version(&bench.screen), version);
        assert_eq!(bench.screen.ui_state().count, Count::Minutes(10));
        bench.after(Duration::from_secs(61));
        assert_ne!(Screen::version(&bench.screen), version);
        assert_eq!(bench.screen.ui_state().count, Count::Minutes(9));
        bench.after(Duration::from_secs(540));
        assert_eq!(bench.screen.ui_state().count, Count::Seconds(60));
        bench.after(Duration::from_millis(581_500));
        assert_eq!(bench.screen.ui_state().count, Count::Seconds(19));
    }

    #[test]
    fn ringing_either_button_stops_it() {
        for button in [Button::Yellow, Button::Long] {
            let mut bench = Bench::new();
            bench.press(Button::Yellow);
            bench.after(Duration::from_secs(600));
            assert_eq!(bench.screen.ui_state().status, "Time is up");
            assert!(*bench.ringer.0.lock().unwrap());
            bench.press(button);
            assert_eq!(bench.timer.state(), TimerState::Set);
            assert!(!*bench.ringer.0.lock().unwrap());
        }
    }

    #[test]
    fn napping_only_yellow_does_anything_and_it_stops() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        assert_eq!((bench.screen.ui_state().status.as_str(), bench.screen.ui_state().hint), ("Napping", "yellow: stop"));
        bench.press(Button::Long);
        bench.input(Input::HoldYellowAndLong);
        assert!(matches!(bench.timer.state(), TimerState::Counting { stretch: Stretch::Nap, .. }));
        bench.press(Button::Yellow);
        assert_eq!(bench.timer.state(), TimerState::Set);
    }

    #[test]
    fn a_nap_over_says_to_wake_up() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        bench.after(Duration::from_secs(20 * 60));
        assert_eq!((bench.screen.ui_state().count, bench.screen.ui_state().status.as_str()), (Count::Seconds(0), "Time to wake up"));
    }

    #[test]
    fn a_pomodoro_says_its_round() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        assert_eq!(bench.screen.ui_state().status, "Work, round 1 of 4");
        bench.after(Duration::from_secs(25 * 60));
        assert_eq!((bench.screen.ui_state().status.as_str(), bench.screen.ui_state().count), ("Break, round 1 of 4", Count::Minutes(5)));
    }
}
