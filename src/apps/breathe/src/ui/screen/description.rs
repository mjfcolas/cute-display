use ui::{Describe, Description};

use super::ui_state::{BreathePage, Session};

impl Describe for BreathePage {
    fn describe(&self) -> Description {
        let mut description = Description::default();
        for row in &self.rhythms {
            description.say_marked("rhythm", &format!("{}: {}", row.name, row.steps), row.mark);
        }
        description.say("status", &self.session.status());
        if let Session::Breathing { volume, .. } = self.session {
            description.say("volume", &format!("{}%", volume.as_percent()));
        }
        description.say("hint", self.hint);
        description
    }
}

#[cfg(test)]
mod tests {
    use domain::sound::Volume;
    use ui::mark::Mark;

    use super::*;
    use crate::domain::breathing::Fullness;
    use crate::ui::screen::ui_state::RhythmRow;

    #[test]
    fn the_page_says_each_rhythm_the_chosen_one_starred_the_status_and_the_volume() {
        let row = |name, steps: &str, mark| RhythmRow { name, steps: steps.into(), mark };
        let page = BreathePage {
            breath: vec![Fullness::EMPTY, Fullness::FULL, Fullness::EMPTY],
            rhythms: [row("Coherence", "in 5, out 5 s", Mark::Plain), row("4-7-8", "in 4, hold 7, out 8 s", Mark::Chosen), row("Square", "in 4, hold 4, out 4, hold 4 s", Mark::Plain)],
            session: Session::Breathing { minutes: 3, volume: Volume::percent(30) },
            hint: "wheel: volume   yellow: stop",
        };
        assert_eq!(
            page.describe().text(),
            [
                "rhythm Coherence: in 5, out 5 s",
                "rhythm 4-7-8: in 4, hold 7, out 8 s *",
                "rhythm Square: in 4, hold 4, out 4, hold 4 s",
                "status Breathing for 3 min",
                "volume 30%",
                "hint wheel: volume yellow: stop"
            ]
        );
    }
}
