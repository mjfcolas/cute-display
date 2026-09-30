use domain::time::TimeOfDay;
use forecast::{DayForecast, HourForecast};
use ui::mark::Mark;

#[derive(Clone, Debug, PartialEq)]
pub enum AlarmUiState {
    Clock(ClockPage),
    Settings(Box<SettingsPage>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockPage {
    pub date: String,
    pub time: Option<TimeOfDay>,
    pub today: Option<DayForecast>,
    pub hours: Vec<HourForecast>,
    pub alarm: String,
    pub hint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPage {
    pub title: String,
    pub days: [SettingRow; 7],
    pub ringtone: SettingRow,
    pub hint: &'static str,
}

/// A value being set shows its part being turned in brackets: `[07]:30`, `[Zen]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    pub name: &'static str,
    pub value: String,
    pub mark: Mark,
}
