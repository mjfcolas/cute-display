use ui::{Describe, Description};

use super::ui_state::TimerPage;

impl Describe for TimerPage {
    fn describe(&self) -> Description {
        let mut description = Description::default();
        for tab in &self.modes {
            description.say_marked("mode", tab.name, tab.mark);
        }
        description.say("count", &format!("{} {}", self.count.number(), self.count.unit()));
        description.say("status", &self.status);
        description.say("hint", self.hint);
        description
    }
}

#[cfg(test)]
mod tests {
    use ui::mark::Mark;

    use super::*;
    use crate::ui::screen::ui_state::{Count, ModeTab};

    #[test]
    fn the_page_says_the_modes_the_chosen_one_starred_the_count_and_the_status() {
        let tab = |name, mark| ModeTab { name, mark };
        let page = TimerPage {
            modes: [tab("Timer", Mark::Plain), tab("Pomodoro", Mark::Chosen), tab("Nap", Mark::Plain)],
            count: Count::Seconds(42),
            status: "Work, round 3 of 4".into(),
            hint: "yellow: pause   long: stop",
        };
        assert_eq!(
            page.describe().text(),
            ["mode Timer", "mode Pomodoro *", "mode Nap", "count 42 s", "status Work, round 3 of 4", "hint yellow: pause long: stop"]
        );
    }
}
