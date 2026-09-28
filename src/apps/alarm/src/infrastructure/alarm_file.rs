use conf_text::ConfText;
use domain::calendar::Weekday;
use domain::files::Files;
use domain::time::TimeOfDay;

use crate::domain::alarm_clock::{AlarmSchedule, AlarmSettings, AlarmSettingsStore, Ringtone};

pub const ALARM_FILE: &str = "alarm.conf";

const ENABLED: &str = "enabled";
const OFF: &str = "off";
const RINGTONE: &str = "ringtone";
const CHIME: &str = "chime";

pub struct AlarmFile {
    files: Box<dyn Files>,
}

impl AlarmFile {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl AlarmSettingsStore for AlarmFile {
    fn load(&mut self) -> Option<AlarmSettings> {
        match self.files.read(ALARM_FILE) {
            Ok(text) => text.map(|text| decode(&ConfText::parse(&text))),
            Err(unavailable) => {
                log::warn!("alarm: {unavailable}; no alarm until it is set again");
                None
            }
        }
    }

    fn save(&mut self, settings: &AlarmSettings) {
        if let Err(unavailable) = self.files.write(ALARM_FILE, &encode(settings)) {
            log::warn!("alarm: {unavailable}; the change holds until the power goes");
        }
    }
}

fn key(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "monday",
        Weekday::Tuesday => "tuesday",
        Weekday::Wednesday => "wednesday",
        Weekday::Thursday => "thursday",
        Weekday::Friday => "friday",
        Weekday::Saturday => "saturday",
        Weekday::Sunday => "sunday",
    }
}

fn encode(settings: &AlarmSettings) -> String {
    let schedule = &settings.schedule;
    let times: Vec<(&str, String)> = Weekday::ALL
        .iter()
        .map(|&day| {
            let time = schedule.time_on(day).map_or_else(|| OFF.into(), |t| format!("{:02}:{:02}", t.hour(), t.minute()));
            (key(day), time)
        })
        .collect();
    let mut pairs = vec![(ENABLED, if schedule.enabled { "yes" } else { "no" })];
    pairs.extend(times.iter().map(|(day, time)| (*day, time.as_str())));
    pairs.push((
        RINGTONE,
        match &settings.ringtone {
            Ringtone::Chime => CHIME,
            Ringtone::Recorded(name) => name,
        },
    ));
    ConfText::render(&pairs)
}

fn decode(conf: &ConfText) -> AlarmSettings {
    let mut schedule = AlarmSchedule::default();
    schedule.enabled = conf.get(ENABLED) == Some("yes");
    for day in Weekday::ALL {
        schedule.set_time_on(day, conf.get(key(day)).and_then(time_of_day));
    }
    let ringtone = match conf.get(RINGTONE) {
        None | Some(CHIME) => Ringtone::Chime,
        Some(name) => Ringtone::Recorded(name.into()),
    };
    AlarmSettings { schedule, ringtone }
}

fn time_of_day(text: &str) -> Option<TimeOfDay> {
    let (hour, minute) = text.split_once(':')?;
    TimeOfDay::new(hour.trim().parse().ok()?, minute.trim().parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory_files::MemoryFiles;

    fn weekdays_at_seven() -> AlarmSettings {
        let mut schedule = AlarmSchedule::default();
        schedule.enabled = true;
        for day in [Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday] {
            schedule.set_time_on(day, TimeOfDay::new(7, 0));
        }
        schedule.set_time_on(Weekday::Saturday, TimeOfDay::new(9, 5));
        AlarmSettings { schedule, ringtone: Ringtone::Recorded("Lost Ark.mp3".into()) }
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let files = MemoryFiles::default();
        let mut file = AlarmFile::new(Box::new(files.clone()));
        file.save(&weekdays_at_seven());
        assert_eq!(file.load(), Some(weekdays_at_seven()));
        assert!(files.read(ALARM_FILE).unwrap().is_some());
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        assert_eq!(
            encode(&weekdays_at_seven()),
            "enabled = yes\nmonday = 07:00\ntuesday = 07:00\nwednesday = 07:00\nthursday = 07:00\nfriday = 07:00\n\
             saturday = 09:05\nsunday = off\nringtone = Lost Ark.mp3\n"
        );
        assert!(encode(&AlarmSettings::default()).ends_with("ringtone = chime\n"));
    }

    #[test]
    fn a_damaged_line_loses_only_its_own_day() {
        let files = MemoryFiles::default();
        files.write(ALARM_FILE, "enabled = yes\nmonday = 25:00\ntuesday = 6:30\nwednesday = soon\n").unwrap();
        let settings = AlarmFile::new(Box::new(files)).load().unwrap();
        assert!(settings.schedule.enabled);
        assert_eq!(settings.schedule.time_on(Weekday::Monday), None);
        assert_eq!(settings.schedule.time_on(Weekday::Tuesday), TimeOfDay::new(6, 30));
        assert_eq!(settings.schedule.time_on(Weekday::Wednesday), None);
        assert_eq!(settings.ringtone, Ringtone::Chime, "no ringtone line is the chime");
    }

    #[test]
    fn no_file_is_no_settings() {
        assert_eq!(AlarmFile::new(Box::new(MemoryFiles::default())).load(), None);
    }
}
