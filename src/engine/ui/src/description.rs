use core::fmt;

use crate::mark::Mark;

/// What a screen shows, in words: a line for each thing a person reads, in the screen's
/// order. Values are said with single spaces, however they are laid out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Description(Vec<Line>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub name: &'static str,
    pub value: String,
    pub mark: Mark,
}

impl Description {
    pub fn say(&mut self, name: &'static str, value: &str) {
        self.say_marked(name, value, Mark::Plain);
    }

    pub fn say_marked(&mut self, name: &'static str, value: &str, mark: Mark) {
        let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
        self.0.push(Line { name, value, mark });
    }

    pub fn followed_by(mut self, more: Description) -> Self {
        self.0.extend(more.0);
        self
    }

    pub fn text(&self) -> Vec<String> {
        self.0.iter().map(Line::to_string).collect()
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)?;
        if !self.value.is_empty() {
            write!(f, " {}", self.value)?;
        }
        if self.mark == Mark::Chosen {
            f.write_str(" *")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_its_name_its_value_with_single_spaces_and_a_star_when_chosen() {
        let mut description = Description::default();
        description.say("hint", "  long: update   wheel: range ");
        description.say_marked("row", "Saturday [07]:00", Mark::Chosen);
        description.say("empty", "");
        assert_eq!(description.text(), ["hint long: update wheel: range", "row Saturday [07]:00 *", "empty"]);
    }

    #[test]
    fn a_description_goes_on_with_another() {
        let mut front = Description::default();
        front.say("front", "alarm");
        let mut screen = Description::default();
        screen.say("page", "clock");
        assert_eq!(front.followed_by(screen).text(), ["front alarm", "page clock"]);
    }
}
