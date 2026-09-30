use forecast::day_weather;
use forecast::units::temperature;
use ui::{Describe, Description};

use super::clock_time;
use super::ui_state::{AlarmUiState, ClockPage, SettingsPage};

impl Describe for AlarmUiState {
    fn describe(&self) -> Description {
        match self {
            AlarmUiState::Clock(page) => describe_clock(page),
            AlarmUiState::Settings(page) => describe_settings(page),
        }
    }
}

fn describe_clock(page: &ClockPage) -> Description {
    let mut description = Description::default();
    description.say("page", "clock");
    description.say("date", &page.date);
    let time = page.time.map_or_else(|| "--:--".into(), clock_time);
    description.say("time", &time);
    if let Some(today) = &page.today {
        description.say("today", &format!("{} {}", today.sky.name(), day_weather::range(today)));
    }
    for hour in &page.hours {
        description.say("hour", &format!("{} {} {}", day_weather::hour_label(hour), hour.sky.name(), temperature(hour.temperature)));
    }
    description.say("alarm", &page.alarm);
    description.say("hint", &page.hint);
    description
}

fn describe_settings(page: &SettingsPage) -> Description {
    let mut description = Description::default();
    description.say("page", "settings");
    description.say("title", &page.title);
    for row in page.days.iter().chain([&page.ringtone]) {
        description.say_marked("row", &format!("{} {}", row.name, row.value), row.mark);
    }
    description.say("hint", page.hint);
    description
}

#[cfg(test)]
mod tests {
    use domain::calendar::Date;
    use domain::time::{LocalTime, TimeOfDay};
    use forecast::{DayForecast, Degrees, HourForecast, Percent, Sky};
    use ui::mark::Mark;

    use super::*;
    use crate::ui::screen::SettingRow;

    #[test]
    fn the_clock_says_the_date_the_time_the_forecast_and_the_alarm() {
        let saturday = Date::new(2026, 9, 26).unwrap();
        let seven = LocalTime { date: saturday, time_of_day: TimeOfDay::new(7, 0).unwrap(), second: 0 };
        let page = ClockPage {
            date: "Saturday 26 September".into(),
            time: TimeOfDay::new(7, 5),
            today: Some(DayForecast { date: saturday, sky: Sky::Rain, low: Degrees(9), high: Degrees(14), rain_chance: Percent::new(90), sunrise: None, sunset: None }),
            hours: vec![HourForecast { start: seven, sky: Sky::Rain, temperature: Degrees(12), rain_chance: None, precipitation: None }],
            alarm: "Alarm tomorrow at 08:30".into(),
            hint: "yellow: alarm on/off   long: settings   wheel: hours".into(),
        };
        let lines = AlarmUiState::Clock(page.clone()).describe().text();
        assert_eq!(
            lines,
            [
                "page clock",
                "date Saturday 26 September",
                "time 07:05",
                "today Rain 9° / 14°",
                "hour 07:00 Rain 12°",
                "alarm Alarm tomorrow at 08:30",
                "hint yellow: alarm on/off long: settings wheel: hours",
            ]
        );
        let unknown = AlarmUiState::Clock(ClockPage { time: None, today: None, hours: vec![], ..page });
        assert_eq!(unknown.describe().text()[2], "time --:--");
    }

    #[test]
    fn the_settings_say_each_row_the_chosen_one_starred() {
        let row = |name, value: &str, mark| SettingRow { name, value: value.into(), mark };
        let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
        let page = SettingsPage {
            title: "Alarm settings (alarm on)".into(),
            days: days.map(|day| if day == "Saturday" { row(day, "[07]:00", Mark::Chosen) } else { row(day, "off", Mark::Plain) }),
            ringtone: row("Ringtone", "Zen", Mark::Plain),
            hint: "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
        };
        let lines = AlarmUiState::Settings(Box::new(page)).describe().text();
        assert_eq!(lines[..3], ["page settings", "title Alarm settings (alarm on)", "row Monday off"]);
        assert_eq!(lines[7..], ["row Saturday [07]:00 *", "row Sunday off", "row Ringtone Zen", "hint wheel: hour, past 23 is off long: minutes yellow: cancel"]);
    }
}
