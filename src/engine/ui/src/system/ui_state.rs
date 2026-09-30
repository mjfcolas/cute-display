#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemUiState {
    pub version: &'static str,
    pub apps: Vec<AppRow>,
    pub settings: Vec<SettingRow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Chosen,
    Plain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppRow {
    pub title: &'static str,
    pub mark: Mark,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    pub name: &'static str,
    pub mark: Mark,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub current: Current,
}

/// Whether a choice is the setting's value now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Current {
    Yes,
    No,
}
