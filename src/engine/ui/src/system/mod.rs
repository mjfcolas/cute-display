mod drawing;
mod ui_state;

use domain::apps::{AppId, Foreground};
use domain::settings::{BacklightDuration, ReadingLamp, Settings};

pub use ui_state::{AppRow, Choice, Current, SettingRow, SystemUiState};

use crate::app_screen::Screen;
use crate::controls::{Button, Input};
use crate::mark::Mark;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Setting {
    Backlight,
    ReadingLamp,
}

impl Setting {
    const ALL: [Self; 2] = [Self::Backlight, Self::ReadingLamp];

    fn name(self) -> &'static str {
        match self {
            Self::Backlight => "Backlight",
            Self::ReadingLamp => "Reading lamp",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfferedApp {
    pub app: AppId,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    App(OfferedApp),
    Setting(Setting),
}

fn backlight_label(duration: BacklightDuration) -> &'static str {
    match duration {
        BacklightDuration::FiveSeconds => "5 s",
        BacklightDuration::TenSeconds => "10 s",
        BacklightDuration::ThirtySeconds => "30 s",
        BacklightDuration::Always => "always",
    }
}

fn current_if(is_current: bool) -> Current {
    if is_current {
        Current::Yes
    } else {
        Current::No
    }
}

fn reading_lamp_label(lamp: ReadingLamp) -> String {
    match lamp {
        ReadingLamp::Off => "off".into(),
        lamp => format!("{}%", lamp.level().as_percent()),
    }
}

pub struct SystemScreen {
    foreground: Foreground,
    settings: Settings,
    version: &'static str,
    offered: Vec<OfferedApp>,
    chosen_row: usize,
}

impl SystemScreen {
    pub fn new(foreground: Foreground, settings: Settings, version: &'static str, offered: Vec<OfferedApp>) -> Self {
        Self { foreground, settings, version, offered, chosen_row: 0 }
    }

    fn rows(&self) -> Vec<Row> {
        self.offered.iter().map(|&app| Row::App(app)).chain(Setting::ALL.map(Row::Setting)).collect()
    }

    fn mark(&self, row: Row) -> Mark {
        if self.rows().get(self.chosen_row) == Some(&row) {
            Mark::Chosen
        } else {
            Mark::Plain
        }
    }

    fn choices(&self, setting: Setting) -> Vec<Choice> {
        match setting {
            Setting::Backlight => {
                let current = self.settings.backlight();
                BacklightDuration::ALL.map(|d| Choice { label: backlight_label(d).into(), current: current_if(d == current) }).into()
            }
            Setting::ReadingLamp => {
                let current = self.settings.reading_lamp();
                ReadingLamp::ALL.map(|lamp| Choice { label: reading_lamp_label(lamp), current: current_if(lamp == current) }).into()
            }
        }
    }
}

impl Screen for SystemScreen {
    type UiState = SystemUiState;

    fn ui_state(&self) -> SystemUiState {
        SystemUiState {
            version: self.version,
            apps: self.offered.iter().map(|&offered| AppRow { title: offered.title, mark: self.mark(Row::App(offered)) }).collect(),
            settings: Setting::ALL
                .map(|setting| SettingRow { name: setting.name(), mark: self.mark(Row::Setting(setting)), choices: self.choices(setting) })
                .into(),
        }
    }

    fn entered(&mut self) {
        let origin = self.foreground.before_system();
        self.chosen_row = self.rows().iter().position(|row| matches!(row, Row::App(offered) if offered.app == origin)).unwrap_or(0);
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => {
                self.chosen_row = (self.chosen_row as i64 + i64::from(detents)).rem_euclid(self.rows().len() as i64) as usize;
            }
            Input::Press(Button::Long) => match self.rows().get(self.chosen_row) {
                Some(Row::App(offered)) => self.foreground.bring_to_front(offered.app),
                Some(Row::Setting(Setting::Backlight)) => self.settings.choose_next_backlight(),
                Some(Row::Setting(Setting::ReadingLamp)) => self.settings.choose_next_reading_lamp(),
                None => {}
            },
            Input::Press(Button::Yellow) => self.foreground.close_system(),
            Input::HoldYellowAndLong => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use domain_testing::settings::StubSettingsStore;

    use super::*;

    const ALARM: OfferedApp = OfferedApp { app: AppId::new("alarm"), title: "Alarm clock" };
    const WEATHER: OfferedApp = OfferedApp { app: AppId::new("weather"), title: "Weather" };
    const RADAR: OfferedApp = OfferedApp { app: AppId::new("radar"), title: "Radar" };
    const OFFERED: [OfferedApp; 3] = [ALARM, WEATHER, RADAR];

    fn opened_from(app: OfferedApp) -> (SystemScreen, Foreground, Settings) {
        let foreground = Foreground::new(app.app);
        let settings = Settings::load(Box::new(StubSettingsStore));
        foreground.open_system();
        let mut screen = SystemScreen::new(foreground.clone(), settings.clone(), "2026.9.0", OFFERED.into());
        Screen::entered(&mut screen);
        (screen, foreground, settings)
    }

    fn input(screen: &mut SystemScreen, input: Input) {
        Screen::on_input(screen, input);
    }

    fn names_on_chosen_rows(state: &SystemUiState) -> Vec<&'static str> {
        let apps = state.apps.iter().filter(|app| app.mark == Mark::Chosen).map(|app| app.title);
        apps.chain(state.settings.iter().filter(|setting| setting.mark == Mark::Chosen).map(|setting| setting.name)).collect()
    }

    fn turn_to(screen: &mut SystemScreen, name: &str) {
        for _ in 0..screen.rows().len() {
            if names_on_chosen_rows(&screen.ui_state()) == [name] {
                return;
            }
            input(screen, Input::Turn(1));
        }
        panic!("no row {name}");
    }

    fn current(setting: &SettingRow) -> Vec<&str> {
        setting.choices.iter().filter(|choice| choice.current == Current::Yes).map(|choice| choice.label.as_str()).collect()
    }

    #[test]
    fn the_app_it_was_opened_from_is_chosen_first() {
        let (screen, _, _) = opened_from(RADAR);
        assert_eq!(names_on_chosen_rows(&screen.ui_state()), ["Radar"]);
    }

    #[test]
    fn pressing_on_an_app_brings_it_to_the_front() {
        let (mut screen, foreground, _) = opened_from(WEATHER);
        turn_to(&mut screen, "Radar");
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(foreground.app(), RADAR.app);
    }

    #[test]
    fn pressing_on_a_setting_changes_it_and_stays_in_the_system_app() {
        let (mut screen, foreground, settings) = opened_from(WEATHER);
        turn_to(&mut screen, "Backlight");
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(settings.backlight(), BacklightDuration::ThirtySeconds);
        turn_to(&mut screen, "Reading lamp");
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(settings.reading_lamp(), ReadingLamp::TenPercent);
        assert_eq!(foreground.app(), AppId::SYSTEM);
    }

    #[test]
    fn the_yellow_button_goes_back_without_changing_anything() {
        let (mut screen, foreground, settings) = opened_from(RADAR);
        input(&mut screen, Input::Turn(1));
        input(&mut screen, Input::Press(Button::Yellow));
        assert_eq!(foreground.app(), RADAR.app);
        assert_eq!(settings.backlight(), BacklightDuration::default());
    }

    #[test]
    fn every_app_offered_is_listed_then_the_settings() {
        let state = opened_from(WEATHER).0.ui_state();
        assert_eq!(state.apps.iter().map(|app| app.title).collect::<Vec<_>>(), ["Alarm clock", "Weather", "Radar"]);
        assert_eq!(state.settings.iter().map(|setting| setting.name).collect::<Vec<_>>(), ["Backlight", "Reading lamp"]);
    }

    #[test]
    fn the_version_is_shown() {
        assert_eq!(opened_from(WEATHER).0.ui_state().version, "2026.9.0");
    }

    #[test]
    fn the_chosen_row_goes_through_the_apps_then_the_settings_and_wraps_around_both_ways() {
        let (mut screen, _, _) = opened_from(ALARM);
        let mut seen = Vec::new();
        for _ in 0..5 {
            input(&mut screen, Input::Turn(1));
            seen.extend(names_on_chosen_rows(&screen.ui_state()));
        }
        assert_eq!(seen, ["Weather", "Radar", "Backlight", "Reading lamp", "Alarm clock"]);
        input(&mut screen, Input::Turn(-1));
        assert_eq!(names_on_chosen_rows(&screen.ui_state()), ["Reading lamp"]);
    }

    #[test]
    fn each_setting_lists_its_values_the_current_one_marked() {
        let (mut screen, _, _) = opened_from(WEATHER);
        turn_to(&mut screen, "Reading lamp");
        input(&mut screen, Input::Press(Button::Long));
        let state = screen.ui_state();
        let labels = |setting: &SettingRow| setting.choices.iter().map(|choice| choice.label.clone()).collect::<Vec<_>>();
        let [backlight, lamp] = &state.settings[..] else { panic!("{state:?}") };
        assert_eq!(labels(backlight), ["5 s", "10 s", "30 s", "always"]);
        assert_eq!(current(backlight), ["10 s"]);
        assert_eq!(labels(lamp), ["off", "10%", "30%", "50%", "100%"]);
        assert_eq!(current(lamp), ["10%"]);
    }
}
