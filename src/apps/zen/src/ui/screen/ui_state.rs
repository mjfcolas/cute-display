use domain::time::TimeOfDay;
use domain::sound::Volume;
use ui::mark::Mark;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZenUiState {
    Zen(ZenPage),
    Settings(SettingsPage),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZenPage {
    pub time: Option<TimeOfDay>,
    pub sound: String,
    /// When the next session starts, or how much time the current one has left.
    pub session: String,
    /// During a session.
    pub volume: Option<Volume>,
    pub hint: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPage {
    pub title: &'static str,
    pub rows: [SettingRow; 3],
    pub hint: &'static str,
}

/// While a value is being set, the part that the wheel changes is in brackets: `[22]:30`,
/// `[Rain]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    pub name: &'static str,
    pub value: String,
    pub mark: Mark,
}
