//! The alarm schedule as a few lines of text, in the device's own directory of a storage:
//!
//! ```text
//! enabled = yes
//! monday = 07:00
//! tuesday = off
//! ```
//!
//! A day that is missing or unreadable has no alarm.

use domain::alarm::{AlarmSchedule, AlarmScheduleStore};
use domain::calendar::Weekday;
use domain::time::TimeOfDay;
use hal::storage::FileStorage;

use crate::conf_text::ConfText;

pub const ALARM_FILE: &str = "cute-display/alarm.conf";

const ENABLED: &str = "enabled";
const OFF: &str = "off";

pub struct AlarmFile<S> {
    storage: S,
}

impl<S: FileStorage> AlarmFile<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: FileStorage + Send> AlarmScheduleStore for AlarmFile<S> {
    fn load(&mut self) -> Option<AlarmSchedule> {
        match ConfText::read(&self.storage, ALARM_FILE) {
            Ok(conf) => conf.map(|conf| decode(&conf)),
            Err(fault) => {
                log::warn!("alarm: {fault}; no alarm until it is set again");
                None
            }
        }
    }

    fn save(&mut self, schedule: &AlarmSchedule) {
        if let Err(fault) = self.storage.write(ALARM_FILE, encode(schedule).as_bytes()) {
            log::warn!("alarm: {fault}; the change holds until the power goes");
        }
    }
}

/// For a device with nowhere to keep its schedule: it lasts until the power goes.
pub struct UnkeptAlarm;

impl AlarmScheduleStore for UnkeptAlarm {
    fn load(&mut self) -> Option<AlarmSchedule> {
        None
    }

    fn save(&mut self, _: &AlarmSchedule) {}
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

fn encode(schedule: &AlarmSchedule) -> String {
    let times: Vec<(&str, String)> = Weekday::ALL
        .iter()
        .map(|&day| {
            let time = schedule.time_on(day).map_or_else(|| OFF.into(), |t| format!("{:02}:{:02}", t.hour(), t.minute()));
            (key(day), time)
        })
        .collect();
    let mut pairs = vec![(ENABLED, if schedule.enabled { "yes" } else { "no" })];
    pairs.extend(times.iter().map(|(day, time)| (*day, time.as_str())));
    ConfText::render(&pairs)
}

fn decode(conf: &ConfText) -> AlarmSchedule {
    let mut schedule = AlarmSchedule::default();
    schedule.enabled = conf.get(ENABLED) == Some("yes");
    for day in Weekday::ALL {
        schedule.set_time_on(day, conf.get(key(day)).and_then(time_of_day));
    }
    schedule
}

fn time_of_day(text: &str) -> Option<TimeOfDay> {
    let (hour, minute) = text.split_once(':')?;
    TimeOfDay::new(hour.trim().parse().ok()?, minute.trim().parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    fn weekdays_at_seven() -> AlarmSchedule {
        let mut schedule = AlarmSchedule::default();
        schedule.enabled = true;
        for day in [Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday] {
            schedule.set_time_on(day, TimeOfDay::new(7, 0));
        }
        schedule.set_time_on(Weekday::Saturday, TimeOfDay::new(9, 5));
        schedule
    }

    #[test]
    fn a_schedule_comes_back_as_it_was_saved() {
        let storage = MemoryStorage::default();
        let mut file = AlarmFile::new(storage.clone());
        file.save(&weekdays_at_seven());
        assert_eq!(file.load(), Some(weekdays_at_seven()));
        assert!(storage.contains("cute-display/alarm.conf"));
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        assert_eq!(
            encode(&weekdays_at_seven()),
            "enabled = yes\nmonday = 07:00\ntuesday = 07:00\nwednesday = 07:00\nthursday = 07:00\nfriday = 07:00\n\
             saturday = 09:05\nsunday = off\n"
        );
    }

    #[test]
    fn a_damaged_line_loses_only_its_own_day() {
        let storage = MemoryStorage::with(ALARM_FILE, "enabled = yes\nmonday = 25:00\ntuesday = 6:30\nwednesday = soon\n");
        let schedule = AlarmFile::new(storage).load().unwrap();
        assert!(schedule.enabled);
        assert_eq!(schedule.time_on(Weekday::Monday), None);
        assert_eq!(schedule.time_on(Weekday::Tuesday), TimeOfDay::new(6, 30));
        assert_eq!(schedule.time_on(Weekday::Wednesday), None);
    }

    #[test]
    fn no_file_is_no_schedule() {
        assert_eq!(AlarmFile::new(MemoryStorage::default()).load(), None);
    }
}
