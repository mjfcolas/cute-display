use super::ui_state::{Current, SystemUiState};
use crate::app_screen::Describe;
use crate::description::Description;

impl Describe for SystemUiState {
    fn describe(&self) -> Description {
        let mut description = Description::default();
        description.say("title", self.title);
        description.say("version", self.version);
        for app in &self.apps {
            description.say_marked("app", app.title, app.mark);
        }
        for setting in &self.settings {
            let current = setting.choices.iter().find(|choice| choice.current == Current::Yes).map_or("", |choice| &choice.label);
            description.say_marked("setting", &format!("{} {current}", setting.name), setting.mark);
        }
        description.say("hint", self.hint);
        description
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::Mark;
    use crate::system::ui_state::{AppRow, Choice, SettingRow};

    #[test]
    fn the_apps_then_the_settings_at_their_values_the_chosen_row_starred() {
        let choice = |label: &str, current| Choice { label: label.into(), current };
        let state = SystemUiState {
            title: "System",
            version: "2026.9.0",
            apps: vec![AppRow { title: "Alarm clock", mark: Mark::Plain }, AppRow { title: "Radar", mark: Mark::Chosen }],
            settings: vec![SettingRow { name: "Reading lamp", mark: Mark::Plain, choices: vec![choice("off", Current::No), choice("10%", Current::Yes)] }],
            hint: "yellow: back",
        };
        assert_eq!(state.describe().text(), ["title System", "version 2026.9.0", "app Alarm clock", "app Radar *", "setting Reading lamp 10%", "hint yellow: back"]);
    }
}
