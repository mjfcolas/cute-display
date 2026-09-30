use forecast::units::{millimetres, temperature};
use ui::{Describe, Description};

use super::ui_state::{ShownPage, TodayPage, WeatherUiState, WeekDay};
use super::NO_FORECAST;

impl Describe for WeatherUiState {
    fn describe(&self) -> Description {
        let mut description = Description::default();
        description.say("title", &self.title);
        match &self.page {
            ShownPage::Today(today) => {
                description.say("page", "today");
                match today {
                    Some(today) => describe_today(&mut description, today),
                    None => description.say("forecast", NO_FORECAST),
                }
            }
            ShownPage::Week(days) => {
                description.say("page", "week");
                match days {
                    Some(days) => days.iter().for_each(|day| describe_day(&mut description, day)),
                    None => description.say("forecast", NO_FORECAST),
                }
            }
        }
        description.say("status", &self.status);
        description
    }
}

fn describe_today(description: &mut Description, today: &TodayPage) {
    description.say("now", &format!("{} {}", today.sky.name(), temperature(today.temperature)));
    description.say("feels", &temperature(today.feels_like));
    if let Some(day) = &today.day {
        description.say("day", day);
    }
    description.say("humidity", &today.humidity);
    description.say("pressure", &today.pressure);
    let from = today.wind_from.map_or(String::new(), |from| format!(" from {}", from.abbreviation()));
    description.say("wind", &format!("{}{from}", today.wind));
    description.say("sun", &today.sun);
    for hour in &today.hours {
        let rain = hour.likely_rain.as_deref().unwrap_or_default();
        let fallen = hour.fallen.map_or(String::new(), millimetres);
        description.say("hour", &format!("{} {} {} {rain} {fallen}", hour.hour, hour.sky.name(), hour.temperature));
    }
}

fn describe_day(description: &mut Description, day: &WeekDay) {
    let rain = day.rain.as_deref().unwrap_or_default();
    description.say("day", &format!("{} {} {} {rain}", day.name, day.sky.name(), day.range));
}

#[cfg(test)]
mod tests {
    use domain::place::CompassPoint;
    use forecast::{Degrees, Millimetres, Sky};

    use super::*;
    use crate::ui::screen::HourColumn;

    fn state(page: ShownPage) -> WeatherUiState {
        WeatherUiState { title: "Paris".into(), page, status: "updated just now".into() }
    }

    #[test]
    fn today_says_the_sky_the_details_and_each_hour() {
        let today = TodayPage {
            sky: Sky::Cloudy,
            temperature: Degrees(-12),
            feels_like: Degrees(-18),
            day: Some("-12° / 21° rain 45%".into()),
            humidity: "100%".into(),
            pressure: "1016 hPa".into(),
            wind: "112 km/h".into(),
            wind_from: Some(CompassPoint::NorthWest),
            sun: "07:40-19:43".into(),
            hours: vec![
                HourColumn { hour: "17".into(), sky: Sky::Rain, temperature: "9°".into(), likely_rain: Some("80%".into()), fallen: Some(Millimetres::from_tenths(12)) },
                HourColumn { hour: "18".into(), sky: Sky::Clear, temperature: "8°".into(), likely_rain: None, fallen: None },
            ],
        };
        assert_eq!(
            state(ShownPage::Today(Some(Box::new(today)))).describe().text(),
            [
                "title Paris",
                "page today",
                "now Cloudy -12°",
                "feels -18°",
                "day -12° / 21° rain 45%",
                "humidity 100%",
                "pressure 1016 hPa",
                "wind 112 km/h from NW",
                "sun 07:40-19:43",
                "hour 17 Rain 9° 80% 1.2 mm",
                "hour 18 Clear 8°",
                "status updated just now",
            ]
        );
    }

    #[test]
    fn the_week_says_each_day_and_either_page_says_when_there_is_no_forecast_yet() {
        let day = WeekDay { name: "Sat", sky: Sky::Snow, range: " -2° / 3°".into(), rain: Some("rain  60%".into()) };
        assert_eq!(state(ShownPage::Week(Some(vec![day]))).describe().text(), ["title Paris", "page week", "day Sat Snow -2° / 3° rain 60%", "status updated just now"]);
        assert_eq!(state(ShownPage::Week(None)).describe().text(), ["title Paris", "page week", "forecast No forecast yet", "status updated just now"]);
        assert_eq!(state(ShownPage::Today(None)).describe().text()[2], "forecast No forecast yet");
    }
}
