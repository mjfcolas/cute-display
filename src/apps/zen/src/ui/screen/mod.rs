mod description;
mod drawing;
mod ui_state;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use domain::clock::Clock;
use domain::time::TimeOfDay;
use ui::controls::{Button, Input};
use ui::mark::Mark;
use ui::Screen;

pub use ui_state::{SettingRow, SettingsPage, ZenPage, ZenUiState};

use crate::domain::zen_sessions::{SessionLength, Soundscape, ZenSessions, ZenState};

const FIRST_TIME: Option<TimeOfDay> = TimeOfDay::new(22, 0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Setting {
    Start,
    Length,
    Sound,
}

impl Setting {
    const ALL: [Self; 3] = [Self::Start, Self::Length, Self::Sound];

    /// Round and round.
    fn turned_by(self, detents: i32) -> Self {
        let at = Self::ALL.iter().position(|setting| *setting == self).unwrap_or(0) as i64;
        let turned = (at + i64::from(detents)).rem_euclid(Self::ALL.len() as i64);
        Self::ALL.get(turned as usize).copied().unwrap_or(self)
    }

    fn name(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Length => "Length",
            Self::Sound => "Sound",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    Zen,
    Settings(SettingsStep),
}

/// Where the settings page is: choosing a row, or setting what is on it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SettingsStep {
    Choosing { row: Setting },
    Hour { to_put_back: Option<TimeOfDay> },
    Minute { to_put_back: Option<TimeOfDay> },
    Length { to_put_back: SessionLength },
    Sound { to_put_back: Soundscape },
}

pub struct ZenScreen {
    zen: ZenSessions,
    clock: Clock,
    mode: Mode,
    /// Those to choose from, read when the choice opens.
    soundscapes: Vec<Soundscape>,
}

impl ZenScreen {
    pub fn new(zen: ZenSessions, clock: Clock) -> Self {
        Self { zen, clock, mode: Mode::Zen, soundscapes: Vec::new() }
    }

    fn zen_page(&self) -> ZenPage {
        let settings = self.zen.settings();
        let (session, volume, hint) = match self.zen.state() {
            ZenState::InSession { left } => (format!("{} left", time_left(left)), Some(settings.volume), "wheel: volume   yellow: stop"),
            ZenState::Resting => {
                let session = match settings.starts_at {
                    Some(starts_at) => format!("Every day at {} for {} h", clock_time(starts_at), settings.length.as_hours()),
                    None => format!("No time set; {} h when started", settings.length.as_hours()),
                };
                (session, None, "yellow: start a session   long: settings")
            }
        };
        ZenPage {
            time: self.clock.now().map(|now| now.time_of_day),
            sound: soundscape_name(&settings.soundscape).into(),
            session,
            volume,
            hint,
        }
    }

    fn settings_page(&self, step: &SettingsStep) -> SettingsPage {
        let settings = self.zen.settings();
        let chosen_row = match step {
            SettingsStep::Choosing { row } => *row,
            SettingsStep::Hour { .. } | SettingsStep::Minute { .. } => Setting::Start,
            SettingsStep::Length { .. } => Setting::Length,
            SettingsStep::Sound { .. } => Setting::Sound,
        };
        let row = |setting: Setting, value: String| SettingRow {
            name: setting.name(),
            value,
            mark: if setting == chosen_row { Mark::Chosen } else { Mark::Plain },
        };
        let start = match (settings.starts_at, step) {
            (None, SettingsStep::Hour { .. }) => "[off]".into(),
            (None, _) => "off".into(),
            (Some(time), SettingsStep::Hour { .. }) => format!("[{:02}]:{:02}", time.hour(), time.minute()),
            (Some(time), SettingsStep::Minute { .. }) => format!("{:02}:[{:02}]", time.hour(), time.minute()),
            (Some(time), _) => clock_time(time),
        };
        let length = format!("{} h", settings.length.as_hours());
        let sound = soundscape_name(&settings.soundscape);
        let (length, sound) = match step {
            SettingsStep::Length { .. } => (format!("[{length}]"), sound.into()),
            SettingsStep::Sound { .. } => (length, format!("[{sound}]")),
            _ => (length, sound.into()),
        };
        let hint = match step {
            SettingsStep::Choosing { .. } => "wheel: start, length or sound   long: set   yellow: back",
            SettingsStep::Hour { .. } => "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
            SettingsStep::Minute { .. } => "wheel: minutes   long: done   yellow: cancel",
            SettingsStep::Length { .. } => "wheel: hours   long: done   yellow: cancel",
            SettingsStep::Sound { .. } => "wheel: sound, playing softly   long: done   yellow: cancel",
        };
        SettingsPage {
            title: "Zen settings",
            rows: [row(Setting::Start, start), row(Setting::Length, length), row(Setting::Sound, sound)],
            hint,
        }
    }

    fn choosing(row: Setting) -> Mode {
        Mode::Settings(SettingsStep::Choosing { row })
    }

    /// In a session every other input is ignored, so that nothing half asleep changes a
    /// setting.
    fn on_zen_input(&mut self, input: Input) {
        match (self.zen.state(), input) {
            (ZenState::InSession { .. }, Input::Turn(detents)) => self.zen.turn_volume(detents),
            (ZenState::InSession { .. }, Input::Press(Button::Yellow)) => self.zen.stop(),
            (ZenState::InSession { .. }, _) => {}
            (ZenState::Resting, Input::Press(Button::Yellow)) => self.zen.start(),
            (ZenState::Resting, Input::Press(Button::Long)) => self.mode = Self::choosing(Setting::Start),
            (ZenState::Resting, _) => {}
        }
    }

    fn on_settings_input(&mut self, step: SettingsStep, input: Input) {
        match (step, input) {
            (_, Input::HoldYellowAndLong) => {}
            (SettingsStep::Choosing { row }, Input::Turn(detents)) => {
                self.mode = Self::choosing(row.turned_by(detents));
            }
            (SettingsStep::Choosing { row: Setting::Start }, Input::Press(Button::Long)) => {
                let to_put_back = self.zen.settings().starts_at;
                if to_put_back.is_none() {
                    self.zen.set_start(FIRST_TIME);
                }
                self.mode = Mode::Settings(SettingsStep::Hour { to_put_back });
            }
            (SettingsStep::Choosing { row: Setting::Length }, Input::Press(Button::Long)) => {
                self.mode = Mode::Settings(SettingsStep::Length { to_put_back: self.zen.settings().length });
            }
            (SettingsStep::Choosing { .. }, Input::Press(Button::Long)) => self.choose_sound(),
            (SettingsStep::Choosing { .. }, Input::Press(Button::Yellow)) => self.mode = Mode::Zen,
            (SettingsStep::Hour { .. }, Input::Turn(detents)) => self.turn_hour(detents),
            (SettingsStep::Hour { to_put_back }, Input::Press(Button::Long)) if self.zen.settings().starts_at.is_some() => {
                self.mode = Mode::Settings(SettingsStep::Minute { to_put_back });
            }
            (SettingsStep::Hour { .. } | SettingsStep::Minute { .. }, Input::Press(Button::Long)) => self.mode = Self::choosing(Setting::Start),
            (SettingsStep::Minute { .. }, Input::Turn(detents)) => self.turn_minute(detents),
            (SettingsStep::Hour { to_put_back } | SettingsStep::Minute { to_put_back }, Input::Press(Button::Yellow)) => {
                self.zen.set_start(to_put_back);
                self.mode = Self::choosing(Setting::Start);
            }
            (SettingsStep::Length { .. }, Input::Turn(detents)) => self.zen.set_length(self.zen.settings().length.turned_by(detents)),
            (SettingsStep::Length { .. }, Input::Press(Button::Long)) => self.mode = Self::choosing(Setting::Length),
            (SettingsStep::Length { to_put_back }, Input::Press(Button::Yellow)) => {
                self.zen.set_length(to_put_back);
                self.mode = Self::choosing(Setting::Length);
            }
            (SettingsStep::Sound { .. }, Input::Turn(detents)) => self.turn_sound(detents),
            (SettingsStep::Sound { .. }, Input::Press(Button::Long)) => self.close_sound(),
            (SettingsStep::Sound { to_put_back }, Input::Press(Button::Yellow)) => {
                self.zen.set_soundscape(to_put_back);
                self.close_sound();
            }
        }
    }

    fn turn_hour(&self, detents: i32) {
        let time = self.zen.settings().starts_at;
        let position = time.map_or(0, |t| i32::from(t.hour()) + 1);
        let turned = (position + detents).rem_euclid(25);
        let minute = time.map_or(0, TimeOfDay::minute);
        let hour = u8::try_from(turned - 1).ok();
        self.zen.set_start(hour.and_then(|hour| TimeOfDay::new(hour, minute)));
    }

    fn turn_minute(&self, detents: i32) {
        if let Some(time) = self.zen.settings().starts_at {
            let minute = (i32::from(time.minute()) + detents).rem_euclid(60);
            self.zen.set_start(u8::try_from(minute).ok().and_then(|minute| TimeOfDay::new(time.hour(), minute)));
        }
    }

    fn choose_sound(&mut self) {
        self.soundscapes = self.zen.soundscapes();
        self.mode = Mode::Settings(SettingsStep::Sound { to_put_back: self.zen.settings().soundscape });
        self.zen.preview();
    }

    fn turn_sound(&self, detents: i32) {
        let current = self.zen.settings().soundscape;
        let at = self.soundscapes.iter().position(|soundscape| *soundscape == current).unwrap_or(0) as i64;
        let turned = (at + i64::from(detents)).rem_euclid(self.soundscapes.len().max(1) as i64);
        if let Some(soundscape) = self.soundscapes.get(turned as usize) {
            self.zen.set_soundscape(soundscape.clone());
        }
    }

    fn close_sound(&mut self) {
        self.zen.end_preview();
        self.mode = Self::choosing(Setting::Sound);
    }
}

impl Screen for ZenScreen {
    type UiState = ZenUiState;

    fn ui_state(&self) -> ZenUiState {
        match &self.mode {
            Mode::Zen => ZenUiState::Zen(self.zen_page()),
            Mode::Settings(step) => ZenUiState::Settings(self.settings_page(step)),
        }
    }

    fn entered(&mut self) {
        self.mode = Mode::Zen;
    }

    fn left(&mut self) {
        if let Mode::Settings(SettingsStep::Sound { .. }) = self.mode {
            self.zen.end_preview();
        }
    }

    fn version(&self) -> u64 {
        let minute = self.clock.now().map_or(0, |now| now.seconds_since_epoch().div_euclid(60));
        let left = match self.zen.state() {
            ZenState::InSession { left } => minutes_left(left),
            ZenState::Resting => 0,
        };
        let mut hasher = DefaultHasher::new();
        (self.zen.revision(), minute, left).hash(&mut hasher);
        hasher.finish()
    }

    fn on_input(&mut self, input: Input) {
        if matches!(self.zen.state(), ZenState::InSession { .. }) {
            if let Mode::Settings(SettingsStep::Sound { .. }) = self.mode {
                self.zen.end_preview();
            }
            self.mode = Mode::Zen;
        }
        match self.mode.clone() {
            Mode::Zen => self.on_zen_input(input),
            Mode::Settings(step) => self.on_settings_input(step, input),
        }
    }
}

fn soundscape_name(soundscape: &Soundscape) -> &str {
    match soundscape {
        Soundscape::BrownNoise => "Brown noise",
        Soundscape::Recorded(name) => sound_files::title(name),
    }
}

fn clock_time(time: TimeOfDay) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// A minute begun counts whole.
fn minutes_left(left: Duration) -> u64 {
    left.as_secs().div_ceil(60)
}

fn time_left(left: Duration) -> String {
    let minutes = minutes_left(left);
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes:02} min"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use domain::apps::Foreground;
    use domain::calendar::Date;
    use domain::clock::TimeKeeper;
    use domain::time::UtcTime;
    use domain_testing::time::FakeTimeKeeper;

    use super::*;
    use crate::domain::zen_sessions::{Player, Volume, ZenSettings, ZenSettingsStore};

    struct StubZenSettingsStore;
    impl ZenSettingsStore for StubZenSettingsStore {
        fn load(&mut self) -> Option<ZenSettings> {
            None
        }
        fn save(&mut self, _: &ZenSettings) {}
    }

    #[derive(Clone, Default)]
    struct StubPlayer(Arc<Mutex<Option<Soundscape>>>);
    impl Player for StubPlayer {
        fn recordings(&self) -> Vec<String> {
            vec!["Rain.wav".into(), "Ocean.wav".into()]
        }
        fn play(&mut self, soundscape: &Soundscape, _: Volume) {
            *self.0.lock().unwrap() = Some(soundscape.clone());
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl StubPlayer {
        fn playing(&self) -> Option<Soundscape> {
            self.0.lock().unwrap().clone()
        }
    }

    fn rain() -> Soundscape {
        Soundscape::Recorded("Rain.wav".into())
    }

    fn ocean() -> Soundscape {
        Soundscape::Recorded("Ocean.wav".into())
    }

    /// Saturday 26 September 2026, 21:00 UTC.
    fn saturday_at_nine() -> UtcTime {
        UtcTime::from_unix_seconds(Date::new(2026, 9, 26).unwrap().days_since_epoch() * 86_400 + 21 * 3600)
    }

    struct Bench {
        screen: ZenScreen,
        zen: ZenSessions,
        clock: Clock,
        keeper: FakeTimeKeeper,
        player: StubPlayer,
        start: Instant,
    }

    impl Bench {
        fn new() -> Self {
            let keeper = FakeTimeKeeper::default();
            let clock = keeper.offline_clock();
            let player = StubPlayer::default();
            let zen = ZenSessions::new(Box::new(StubZenSettingsStore), Box::new(player.clone()), Foreground::new(crate::ID));
            let mut screen = ZenScreen::new(zen.clone(), clock.clone());
            Screen::entered(&mut screen);
            let bench = Self { screen, zen, clock, keeper, player, start: Instant::now() };
            bench.after(Duration::ZERO);
            bench
        }

        fn after(&self, elapsed: Duration) {
            let seconds = i64::try_from(elapsed.as_secs()).unwrap();
            self.keeper.clone().set(UtcTime::from_unix_seconds(saturday_at_nine().unix_seconds() + seconds));
            let now = self.start + elapsed;
            self.clock.tick(now);
            self.zen.tick(now, self.clock.now());
        }

        fn input(&mut self, input: Input) {
            Screen::on_input(&mut self.screen, input);
        }

        fn press(&mut self, control: Button) {
            self.input(Input::Press(control));
        }

        fn zen_page(&self) -> ZenPage {
            match self.screen.ui_state() {
                ZenUiState::Zen(page) => page,
                ZenUiState::Settings(page) => panic!("the settings are shown: {page:?}"),
            }
        }

        fn settings_page(&self) -> SettingsPage {
            match self.screen.ui_state() {
                ZenUiState::Settings(page) => page,
                ZenUiState::Zen(page) => panic!("the zen page is shown: {page:?}"),
            }
        }

        fn chosen_row(&self) -> &'static str {
            let chosen: Vec<&'static str> = self.settings_page().rows.into_iter().filter(|row| row.mark == Mark::Chosen).map(|row| row.name).collect();
            let [row] = chosen[..] else { panic!("rows chosen: {chosen:?}") };
            row
        }

        fn value_of(&self, name: &str) -> String {
            self.settings_page().rows.into_iter().find(|row| row.name == name).map(|row| row.value).unwrap()
        }
    }

    #[test]
    fn the_yellow_button_starts_a_session_and_stops_it() {
        let mut bench = Bench::new();
        assert_eq!(bench.zen_page().hint, "yellow: start a session   long: settings");
        bench.press(Button::Yellow);
        assert!(matches!(bench.zen.state(), ZenState::InSession { .. }));
        assert_eq!((bench.zen_page().session.as_str(), bench.zen_page().volume), ("1 h left", Some(Volume::percent(30))));
        assert_eq!(bench.zen_page().hint, "wheel: volume   yellow: stop");
        bench.press(Button::Yellow);
        assert_eq!(bench.zen.state(), ZenState::Resting);
        assert_eq!(bench.zen_page().volume, None);
    }

    #[test]
    fn in_a_session_the_wheel_turns_the_volume_and_the_long_button_does_nothing() {
        let mut bench = Bench::new();
        bench.press(Button::Yellow);
        bench.input(Input::Turn(2));
        assert_eq!(bench.zen.settings().volume, Volume::percent(40));
        bench.press(Button::Long);
        bench.input(Input::HoldYellowAndLong);
        assert!(matches!(bench.screen.ui_state(), ZenUiState::Zen(_)));
        assert!(matches!(bench.zen.state(), ZenState::InSession { .. }));
    }

    #[test]
    fn the_time_left_counts_down_by_the_minute() {
        let mut bench = Bench::new();
        bench.press(Button::Yellow);
        let version = Screen::version(&bench.screen);
        bench.after(Duration::from_secs(30));
        assert_eq!(Screen::version(&bench.screen), version);
        assert_eq!(bench.zen_page().session, "1 h left");
        bench.after(Duration::from_secs(61));
        assert_ne!(Screen::version(&bench.screen), version);
        assert_eq!(bench.zen_page().session, "59 min left");
        assert_eq!(time_left(Duration::from_secs(5 * 3600 + 7 * 60)), "5 h 07 min");
    }

    #[test]
    fn the_start_is_set_hour_then_minutes_and_past_23_is_off() {
        let mut bench = Bench::new();
        assert_eq!(bench.zen_page().session, "No time set; 1 h when started");
        bench.press(Button::Long);
        assert_eq!(bench.chosen_row(), "Start");
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Start"), "[22]:00");
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        bench.input(Input::Turn(-15));
        assert_eq!(bench.value_of("Start"), "23:[45]");
        bench.press(Button::Long);
        assert_eq!(bench.zen.settings().starts_at, TimeOfDay::new(23, 45));
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        assert_eq!(bench.value_of("Start"), "[off]");
        bench.press(Button::Long);
        assert_eq!(bench.chosen_row(), "Start", "an off start has no minutes to set");
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        assert_eq!(bench.zen.settings().starts_at, None, "going back puts it back");
        bench.press(Button::Yellow);
        assert_eq!(bench.zen_page().session, "No time set; 1 h when started");
    }

    #[test]
    fn the_length_is_set_in_hours_and_going_back_puts_it_back() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Length"), "[1 h]");
        bench.input(Input::Turn(20));
        assert_eq!(bench.value_of("Length"), "[12 h]");
        bench.press(Button::Long);
        assert_eq!(bench.zen.settings().length.as_hours(), 12);
        bench.press(Button::Long);
        bench.input(Input::Turn(-3));
        bench.press(Button::Yellow);
        assert_eq!(bench.zen.settings().length.as_hours(), 12);
        assert_eq!(bench.chosen_row(), "Length");
    }

    #[test]
    fn the_wheel_goes_round_the_sounds_each_playing_softly_until_done() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(-1));
        assert_eq!(bench.chosen_row(), "Sound");
        bench.press(Button::Long);
        assert_eq!(bench.player.playing(), Some(Soundscape::BrownNoise));
        bench.input(Input::Turn(-1));
        assert_eq!((bench.zen.settings().soundscape, bench.player.playing()), (ocean(), Some(ocean())));
        assert_eq!(bench.value_of("Sound"), "[Ocean]");
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        assert_eq!((bench.zen.settings().soundscape, bench.player.playing()), (rain(), None));
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        bench.press(Button::Yellow);
        assert_eq!((bench.zen.settings().soundscape, bench.player.playing()), (rain(), None), "going back puts it back");
    }

    #[test]
    fn leaving_the_app_while_choosing_a_sound_stops_it() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        Screen::left(&mut bench.screen);
        assert_eq!(bench.player.playing(), None);
    }

    #[test]
    fn a_session_starting_shows_the_zen_page_whatever_was_set() {
        let mut bench = Bench::new();
        bench.zen.set_start(TimeOfDay::new(21, 1));
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.after(Duration::from_secs(60));
        bench.input(Input::Turn(1));
        assert!(matches!(bench.screen.ui_state(), ZenUiState::Zen(_)));
        assert_eq!(bench.zen.settings().soundscape, Soundscape::BrownNoise, "the wheel turned the volume, not the sound");
        assert_eq!(bench.zen.settings().volume, Volume::percent(35));
        assert_eq!(bench.zen_page().sound, "Brown noise");
    }

    #[test]
    fn the_next_session_is_said_with_its_length() {
        let bench = Bench::new();
        bench.zen.set_start(TimeOfDay::new(22, 30));
        bench.zen.set_length(SessionLength::hours(8).unwrap());
        assert_eq!(bench.zen_page().session, "Every day at 22:30 for 8 h");
        assert_eq!(bench.zen_page().time, TimeOfDay::new(21, 0));
    }
}
