use conf_text::ConfText;
use domain::files::Files;

use crate::domain::breathing::{BreathingSettings, BreathingSettingsStore, Rhythm, Volume};

pub const BREATHE_FILE: &str = "breathe.conf";

const RHYTHM: &str = "rhythm";
const VOLUME: &str = "volume";

pub struct BreatheFile {
    files: Box<dyn Files>,
}

impl BreatheFile {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl BreathingSettingsStore for BreatheFile {
    fn load(&mut self) -> Option<BreathingSettings> {
        match self.files.read(BREATHE_FILE) {
            Ok(text) => text.map(|text| decode(&ConfText::parse(&text))),
            Err(unavailable) => {
                log::warn!("breathe: {unavailable}; the first rhythm");
                None
            }
        }
    }

    fn save(&mut self, settings: &BreathingSettings) {
        if let Err(unavailable) = self.files.write(BREATHE_FILE, &encode(settings)) {
            log::warn!("breathe: {unavailable}; the change holds until the power goes");
        }
    }
}

fn rhythm_name(rhythm: Rhythm) -> &'static str {
    match rhythm {
        Rhythm::Coherence => "coherence",
        Rhythm::FourSevenEight => "4-7-8",
        Rhythm::Square => "square",
    }
}

fn encode(settings: &BreathingSettings) -> String {
    let volume = settings.volume.as_percent().to_string();
    ConfText::render(&[(RHYTHM, rhythm_name(settings.rhythm)), (VOLUME, &volume)])
}

/// A line that is missing or damaged keeps its default.
fn decode(conf: &ConfText) -> BreathingSettings {
    let default = BreathingSettings::default();
    BreathingSettings {
        rhythm: Rhythm::ALL.into_iter().find(|rhythm| conf.get(RHYTHM).map(str::trim) == Some(rhythm_name(*rhythm))).unwrap_or(default.rhythm),
        volume: conf.get(VOLUME).and_then(|volume| volume.trim().parse().ok()).map_or(default.volume, Volume::percent),
    }
}

#[cfg(test)]
mod tests {
    use domain_testing::files::FakeFiles;

    use super::*;

    fn falling_asleep() -> BreathingSettings {
        BreathingSettings { rhythm: Rhythm::FourSevenEight, volume: Volume::percent(15) }
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let files = FakeFiles::default();
        let mut file = BreatheFile::new(Box::new(files.clone()));
        file.save(&falling_asleep());
        assert_eq!(file.load(), Some(falling_asleep()));
        assert!(files.read(BREATHE_FILE).unwrap().is_some());
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        assert_eq!(encode(&falling_asleep()), "rhythm = 4-7-8\nvolume = 15\n");
        assert_eq!(encode(&BreathingSettings::default()), "rhythm = coherence\nvolume = 30\n");
    }

    #[test]
    fn a_damaged_line_keeps_its_default() {
        let files = FakeFiles::with(BREATHE_FILE, "rhythm = fast\nvolume = loud\n");
        assert_eq!(BreatheFile::new(Box::new(files)).load(), Some(BreathingSettings::default()));
    }

    #[test]
    fn no_file_is_no_settings() {
        assert_eq!(BreatheFile::new(Box::new(FakeFiles::default())).load(), None);
    }
}
