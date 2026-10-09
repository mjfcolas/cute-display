use ui::mark::Mark;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerPage {
    pub modes: [ModeTab; 3],
    pub count: Count,
    /// What is counted down, or what it does once it starts.
    pub status: String,
    pub hint: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModeTab {
    pub name: &'static str,
    pub mark: Mark,
}

/// Seconds only during the last minute, so that before that, the panel redraws only once a
/// minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Count {
    Minutes(u16),
    Seconds(u8),
}

impl Count {
    pub fn number(self) -> u16 {
        match self {
            Self::Minutes(minutes) => minutes,
            Self::Seconds(seconds) => u16::from(seconds),
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::Minutes(_) => "min",
            Self::Seconds(_) => "s",
        }
    }
}
