use domain::sound::Volume;

use crate::domain::breathing::Fullness;
use ui::mark::Mark;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreathePage {
    /// Spread evenly over one breath of the chosen rhythm.
    pub breath: Vec<Fullness>,
    pub rhythms: [RhythmRow; 3],
    pub session: Session,
    pub hint: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Session {
    Resting,
    Breathing { minutes: u64, volume: Volume },
}

impl Session {
    pub fn status(self) -> String {
        match self {
            Self::Resting => "Until you stop it".into(),
            Self::Breathing { minutes: 0, .. } => "Breathing".into(),
            Self::Breathing { minutes, .. } => format!("Breathing for {minutes} min"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhythmRow {
    pub name: &'static str,
    /// Its steps and their lengths: `in 4, hold 7, out 8 s`.
    pub steps: String,
    pub mark: Mark,
}
