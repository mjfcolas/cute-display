use conf_text::ConfText;
use domain::files::Files;
use domain::time::TimeOfDay;

use crate::domain::zen_sessions::{SessionLength, Soundscape, Volume, ZenSettings, ZenSettingsStore};

pub const ZEN_FILE: &str = "zen.conf";

const START: &str = "start";
const HOURS: &str = "hours";
const SOUND: &str = "sound";
const VOLUME: &str = "volume";
const OFF: &str = "off";
const BROWN_NOISE: &str = "brown noise";

pub struct ZenFile {
    files: Box<dyn Files>,
}

impl ZenFile {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl ZenSettingsStore for ZenFile {
    fn load(&mut self) -> Option<ZenSettings> {
        match self.files.read(ZEN_FILE) {
            Ok(text) => text.map(|text| decode(&ConfText::parse(&text))),
            Err(unavailable) => {
                log::warn!("zen: {unavailable}; no session until it is set again");
                None
            }
        }
    }

    fn save(&mut self, settings: &ZenSettings) {
        if let Err(unavailable) = self.files.write(ZEN_FILE, &encode(settings)) {
            log::warn!("zen: {unavailable}; the change holds until the power goes");
        }
    }
}

fn encode(settings: &ZenSettings) -> String {
    let start = settings.starts_at.map_or_else(|| OFF.into(), |t| format!("{:02}:{:02}", t.hour(), t.minute()));
    let hours = settings.length.as_hours().to_string();
    let sound = match &settings.soundscape {
        Soundscape::BrownNoise => BROWN_NOISE,
        Soundscape::Recorded(name) => name,
    };
    let volume = settings.volume.as_percent().to_string();
    ConfText::render(&[(START, start.as_str()), (HOURS, hours.as_str()), (SOUND, sound), (VOLUME, volume.as_str())])
}

/// A line that is missing or damaged keeps its default.
fn decode(conf: &ConfText) -> ZenSettings {
    let default = ZenSettings::default();
    ZenSettings {
        starts_at: conf.get(START).and_then(time_of_day),
        length: conf.get(HOURS).and_then(|hours| SessionLength::hours(hours.trim().parse().ok()?)).unwrap_or(default.length),
        soundscape: match conf.get(SOUND) {
            None | Some(BROWN_NOISE) => Soundscape::BrownNoise,
            Some(name) => Soundscape::Recorded(name.into()),
        },
        volume: conf.get(VOLUME).and_then(|volume| volume.trim().parse().ok()).map_or(default.volume, Volume::percent),
    }
}

fn time_of_day(text: &str) -> Option<TimeOfDay> {
    let (hour, minute) = text.split_once(':')?;
    TimeOfDay::new(hour.trim().parse().ok()?, minute.trim().parse().ok()?)
}

#[cfg(test)]
mod tests {
    use domain_testing::files::FakeFiles;

    use super::*;

    fn rain_at_ten() -> ZenSettings {
        ZenSettings {
            starts_at: TimeOfDay::new(22, 0),
            length: SessionLength::hours(8).unwrap(),
            soundscape: Soundscape::Recorded("Rain.wav".into()),
            volume: Volume::percent(45),
        }
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let files = FakeFiles::default();
        let mut file = ZenFile::new(Box::new(files.clone()));
        file.save(&rain_at_ten());
        assert_eq!(file.load(), Some(rain_at_ten()));
        assert!(files.read(ZEN_FILE).unwrap().is_some());
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        assert_eq!(encode(&rain_at_ten()), "start = 22:00\nhours = 8\nsound = Rain.wav\nvolume = 45\n");
        assert_eq!(encode(&ZenSettings::default()), "start = off\nhours = 1\nsound = brown noise\nvolume = 30\n");
    }

    #[test]
    fn a_damaged_line_keeps_its_default() {
        let files = FakeFiles::with(ZEN_FILE, "start = 25:00\nhours = 13\nvolume = loud\n");
        assert_eq!(ZenFile::new(Box::new(files)).load(), Some(ZenSettings::default()));
    }

    #[test]
    fn no_file_is_no_settings() {
        assert_eq!(ZenFile::new(Box::new(FakeFiles::default())).load(), None);
    }
}
