use ui::{Describe, Description};

use super::clock_time;
use super::ui_state::{SettingsPage, ZenPage, ZenUiState};

impl Describe for ZenUiState {
    fn describe(&self) -> Description {
        match self {
            ZenUiState::Zen(page) => describe_zen(page),
            ZenUiState::Settings(page) => describe_settings(page),
        }
    }
}

fn describe_zen(page: &ZenPage) -> Description {
    let mut description = Description::default();
    description.say("page", "zen");
    description.say("time", &page.time.map_or_else(|| "--:--".into(), clock_time));
    description.say("sound", &page.sound);
    description.say("session", &page.session);
    if let Some(volume) = page.volume {
        description.say("volume", &format!("{}%", volume.as_percent()));
    }
    description.say("hint", page.hint);
    description
}

fn describe_settings(page: &SettingsPage) -> Description {
    let mut description = Description::default();
    description.say("page", "settings");
    description.say("title", page.title);
    for row in &page.rows {
        description.say_marked("row", &format!("{} {}", row.name, row.value), row.mark);
    }
    description.say("hint", page.hint);
    description
}

#[cfg(test)]
mod tests {
    use domain::time::TimeOfDay;
    use domain::sound::Volume;
    use ui::mark::Mark;

    use super::*;
    use crate::ui::screen::SettingRow;

    #[test]
    fn the_zen_page_says_the_time_the_sound_and_the_session() {
        let page = ZenPage {
            time: TimeOfDay::new(22, 5),
            sound: "Rain".into(),
            session: "Every day at 22:30 for 2 h".into(),
            volume: None,
            hint: "yellow: start a session   long: settings",
        };
        assert_eq!(
            ZenUiState::Zen(page.clone()).describe().text(),
            ["page zen", "time 22:05", "sound Rain", "session Every day at 22:30 for 2 h", "hint yellow: start a session long: settings"]
        );
        let in_session = ZenPage { time: None, session: "2 h left".into(), volume: Some(Volume::percent(30)), ..page };
        assert_eq!(ZenUiState::Zen(in_session).describe().text()[1..5], ["time --:--", "sound Rain", "session 2 h left", "volume 30%"]);
    }

    #[test]
    fn the_settings_say_each_row_the_chosen_one_starred() {
        let row = |name, value: &str, mark| SettingRow { name, value: value.into(), mark };
        let page = SettingsPage {
            title: "Zen settings",
            rows: [row("Start", "[22]:30", Mark::Chosen), row("Length", "2 h", Mark::Plain), row("Sound", "Rain", Mark::Plain)],
            hint: "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
        };
        assert_eq!(
            ZenUiState::Settings(page).describe().text(),
            ["page settings", "title Zen settings", "row Start [22]:30 *", "row Length 2 h", "row Sound Rain", "hint wheel: hour, past 23 is off long: minutes yellow: cancel"]
        );
    }
}
