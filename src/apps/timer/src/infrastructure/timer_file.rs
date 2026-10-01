use conf_text::ConfText;
use domain::files::Files;

use crate::domain::timer::{Minutes, Mode, TimerSettings, TimerSettingsStore};

pub const TIMER_FILE: &str = "timer.conf";

const MODE: &str = "mode";
const COUNTDOWN: &str = "countdown";
const POMODORO: &str = "pomodoro";
const NAP: &str = "nap";

pub struct TimerFile {
    files: Box<dyn Files>,
}

impl TimerFile {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl TimerSettingsStore for TimerFile {
    fn load(&mut self) -> Option<TimerSettings> {
        match self.files.read(TIMER_FILE) {
            Ok(text) => text.map(|text| decode(&ConfText::parse(&text))),
            Err(unavailable) => {
                log::warn!("timer: {unavailable}; the lengths it starts with");
                None
            }
        }
    }

    fn save(&mut self, settings: &TimerSettings) {
        if let Err(unavailable) = self.files.write(TIMER_FILE, &encode(settings)) {
            log::warn!("timer: {unavailable}; the change holds until the power goes");
        }
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Countdown => COUNTDOWN,
        Mode::Pomodoro => POMODORO,
        Mode::Nap => NAP,
    }
}

fn encode(settings: &TimerSettings) -> String {
    let [countdown, pomodoro, nap] = Mode::ALL.map(|mode| settings.length_of(mode).as_minutes().to_string());
    ConfText::render(&[(MODE, mode_name(settings.mode)), (COUNTDOWN, &countdown), (POMODORO, &pomodoro), (NAP, &nap)])
}

/// A line missing or damaged keeps its default.
fn decode(conf: &ConfText) -> TimerSettings {
    let default = TimerSettings::default();
    let length = |mode: Mode| {
        let lengths = mode.lengths();
        conf.get(mode_name(mode))
            .and_then(|minutes| minutes.trim().parse().ok())
            .map(Minutes::new)
            .filter(|minutes| lengths.contains(minutes))
            .unwrap_or(default.length_of(mode))
    };
    TimerSettings {
        mode: Mode::ALL.into_iter().find(|mode| conf.get(MODE).map(str::trim) == Some(mode_name(*mode))).unwrap_or(default.mode),
        countdown: length(Mode::Countdown),
        pomodoro: length(Mode::Pomodoro),
        nap: length(Mode::Nap),
    }
}

#[cfg(test)]
mod tests {
    use domain_testing::files::FakeFiles;

    use super::*;

    fn napping() -> TimerSettings {
        TimerSettings { mode: Mode::Nap, countdown: Minutes::new(3), pomodoro: Minutes::new(50), nap: Minutes::new(25) }
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let files = FakeFiles::default();
        let mut file = TimerFile::new(Box::new(files.clone()));
        file.save(&napping());
        assert_eq!(file.load(), Some(napping()));
        assert!(files.read(TIMER_FILE).unwrap().is_some());
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        assert_eq!(encode(&napping()), "mode = nap\ncountdown = 3\npomodoro = 50\nnap = 25\n");
        assert_eq!(encode(&TimerSettings::default()), "mode = countdown\ncountdown = 10\npomodoro = 25\nnap = 20\n");
    }

    #[test]
    fn a_damaged_line_keeps_its_default() {
        let files = FakeFiles::with(TIMER_FILE, "mode = egg\ncountdown = 0\npomodoro = 61\nnap = long\n");
        assert_eq!(TimerFile::new(Box::new(files)).load(), Some(TimerSettings::default()));
    }

    #[test]
    fn no_file_is_no_settings() {
        assert_eq!(TimerFile::new(Box::new(FakeFiles::default())).load(), None);
    }
}
