//! Open-Meteo gives times already in the place's own time zone.

use domain::calendar::Date;
use domain::fetch::Unavailable;
use domain::internet::Internet;
use domain::place::Place;
use domain::time::{LocalTime, TimeOfDay};
use serde::Deserialize;

use crate::{
    CompassPoint, DayForecast, Degrees, Forecast, ForecastSource, Hectopascals, HourForecast, KilometresPerHour, Millimetres,
    Percent, Sky, Today, Wind,
};

const DAYS: usize = 7;
/// Counted from the hour under way; a day's worth, so that an old forecast still has hours
/// ahead.
const HOURS: usize = 24;

pub struct OpenMeteo<I> {
    internet: I,
}

impl<I: Internet> OpenMeteo<I> {
    pub fn new(internet: I) -> Self {
        Self { internet }
    }
}

impl<I: Internet> ForecastSource for OpenMeteo<I> {
    fn fetch(&mut self, place: &Place) -> Result<Forecast, Unavailable> {
        let body = self.internet.get(&url(place))?;
        read_forecast(&body)
    }
}

fn url(place: &Place) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.4}&longitude={:.4}\
         &current=temperature_2m,weather_code,apparent_temperature,relative_humidity_2m,pressure_msl,\
         wind_speed_10m,wind_direction_10m\
         &hourly=temperature_2m,weather_code,precipitation_probability,precipitation&forecast_hours={HOURS}\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,sunrise,sunset\
         &timezone=auto&forecast_days={DAYS}",
        place.point.latitude, place.point.longitude
    )
}

#[derive(Deserialize)]
struct Answer {
    current: Current,
    hourly: Hourly,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: u16,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    pressure_msl: f64,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

/// A column per value, a row per hour.
#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    weather_code: Vec<u16>,
    temperature_2m: Vec<f64>,
    precipitation_probability: Vec<Option<f64>>,
    precipitation: Vec<Option<f64>>,
}

/// A column per value, a row per day.
#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<u16>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    precipitation_probability_max: Vec<Option<f64>>,
    sunrise: Vec<Option<String>>,
    sunset: Vec<Option<String>>,
}

fn read_forecast(body: &[u8]) -> Result<Forecast, Unavailable> {
    let answer: Answer = serde_json::from_slice(body).map_err(|e| Unavailable(format!("unreadable forecast: {e}")))?;
    let daily = answer.daily;
    let week = (0..daily.time.len())
        .map(|n| {
            let (sunrise, sunset) = sun_times(daily.sunrise.get(n), daily.sunset.get(n));
            Ok(DayForecast {
                date: date_of(row(&daily.time, n)?)?,
                sky: sky_of(*row(&daily.weather_code, n)?)?,
                low: degrees(*row(&daily.temperature_2m_min, n)?),
                high: degrees(*row(&daily.temperature_2m_max, n)?),
                rain_chance: daily.precipitation_probability_max.get(n).copied().flatten().map(percent),
                sunrise,
                sunset,
            })
        })
        .collect::<Result<Vec<_>, Unavailable>>()?;
    if week.is_empty() {
        return Err(Unavailable("the forecast has no days".into()));
    }
    let current = answer.current;
    let today = Today {
        sky: sky_of(current.weather_code)?,
        now: degrees(current.temperature_2m),
        feels_like: degrees(current.apparent_temperature),
        humidity: percent(current.relative_humidity_2m),
        pressure: Hectopascals(whole(current.pressure_msl)),
        wind: Wind {
            speed: KilometresPerHour(whole(current.wind_speed_10m)),
            from: CompassPoint::from_degrees(whole(current.wind_direction_10m)),
        },
    };
    let hourly = answer.hourly;
    let hours = (0..hourly.time.len())
        .map(|n| {
            Ok(HourForecast {
                start: time_of(row(&hourly.time, n)?)?,
                sky: sky_of(*row(&hourly.weather_code, n)?)?,
                temperature: degrees(*row(&hourly.temperature_2m, n)?),
                rain_chance: hourly.precipitation_probability.get(n).copied().flatten().map(percent),
                precipitation: hourly.precipitation.get(n).copied().flatten().map(millimetres),
            })
        })
        .collect::<Result<Vec<_>, Unavailable>>()?;
    Ok(Forecast { today, hours, week })
}

/// Where the sun neither rises nor sets, Open-Meteo gives both at the same midnight.
fn sun_times(rise: Option<&Option<String>>, set: Option<&Option<String>>) -> (Option<TimeOfDay>, Option<TimeOfDay>) {
    let read = |time: Option<&Option<String>>| time?.as_deref().and_then(|time| time_of(time).ok());
    match (read(rise), read(set)) {
        (Some(rise), Some(set)) if rise == set => (None, None),
        (rise, set) => (rise.map(|time| time.time_of_day), set.map(|time| time.time_of_day)),
    }
}

fn row<T>(column: &[T], n: usize) -> Result<&T, Unavailable> {
    column.get(n).ok_or_else(|| Unavailable("the forecast's columns differ in length".into()))
}

fn percent(value: f64) -> Percent {
    Percent::saturating(value.round().clamp(0.0, 100.0) as u8)
}

fn millimetres(value: f64) -> Millimetres {
    Millimetres::from_tenths((value * 10.0).round().clamp(0.0, f64::from(u16::MAX)) as u16)
}

fn whole(value: f64) -> u16 {
    value.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

fn degrees(celsius: f64) -> Degrees {
    Degrees(celsius.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16)
}

/// `2026-09-25`.
fn date_of(text: &str) -> Result<Date, Unavailable> {
    let mut parts = text.splitn(3, '-');
    let date = (|| Date::new(parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))();
    date.ok_or_else(|| Unavailable(format!("unreadable date '{text}'")))
}

/// `2026-09-25T17:00`.
fn time_of(text: &str) -> Result<LocalTime, Unavailable> {
    let unreadable = || Unavailable(format!("unreadable time '{text}'"));
    let (date, time) = text.split_once('T').ok_or_else(unreadable)?;
    let (hour, minute) = time.split_once(':').ok_or_else(unreadable)?;
    let time_of_day = TimeOfDay::new(hour.parse().map_err(|_| unreadable())?, minute.parse().map_err(|_| unreadable())?);
    Ok(LocalTime { date: date_of(date)?, time_of_day: time_of_day.ok_or_else(unreadable)?, second: 0 })
}

/// WMO weather interpretation codes, as Open-Meteo documents them.
fn sky_of(code: u16) -> Result<Sky, Unavailable> {
    Ok(match code {
        0 => Sky::Clear,
        1 | 2 => Sky::PartlyCloudy,
        3 => Sky::Cloudy,
        45 | 48 => Sky::Fog,
        51..=57 | 61..=67 | 80..=82 => Sky::Rain,
        71..=77 | 85 | 86 => Sky::Snow,
        95..=99 => Sky::Storm,
        other => return Err(Unavailable(format!("unknown weather code {other}"))),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::calendar::Weekday;
    use domain::internet::BodyReader;
    use domain::place::GeoPoint;

    use super::*;

    /// Shaped like a real answer, trimmed of the fields not asked for.
    const ANSWER: &str = r#"{
        "latitude": 48.86, "longitude": 2.3399997, "timezone": "Europe/Paris",
        "current_units": {"time": "iso8601", "temperature_2m": "°C", "weather_code": "wmo code"},
        "current": {
            "time": "2026-09-25T17:45", "interval": 900, "temperature_2m": 18.6, "weather_code": 3,
            "apparent_temperature": 17.4, "relative_humidity_2m": 64, "pressure_msl": 1015.6,
            "wind_speed_10m": 12.4, "wind_direction_10m": 214
        },
        "hourly_units": {"time": "iso8601", "temperature_2m": "°C", "weather_code": "wmo code"},
        "hourly": {
            "time": ["2026-09-25T17:00", "2026-09-25T18:00", "2026-09-25T19:00"],
            "temperature_2m": [18.6, 17.4, 15.5],
            "weather_code": [3, 61, 0],
            "precipitation_probability": [5, 70, null],
            "precipitation": [0.0, 2.35, null]
        },
        "daily_units": {"time": "iso8601", "weather_code": "wmo code"},
        "daily": {
            "time": ["2026-09-25", "2026-09-26", "2026-09-27", "2026-09-28", "2026-09-29", "2026-09-30", "2026-10-01"],
            "weather_code": [3, 61, 2, 0, 45, 71, 95],
            "temperature_2m_max": [21.4, 17.0, 19.2, 22.5, 16.1, 3.4, 18.0],
            "temperature_2m_min": [11.6, 12.3, 9.8, 10.1, 8.0, -2.5, 11.9],
            "precipitation_probability_max": [10, 85, 20, 0, 5, 60, null],
            "sunrise": ["2026-09-25T07:40", "2026-09-26T07:41", "2026-09-27T07:43", "2026-09-28T07:44",
                        "2026-09-29T07:46", "2026-09-30T07:47", "2026-10-01T07:49"],
            "sunset": ["2026-09-25T19:43", "2026-09-26T19:41", "2026-09-27T19:39", "2026-09-28T19:36",
                       "2026-09-29T19:34", "2026-09-30T19:32", "2026-10-01T19:30"]
        }
    }"#;

    struct Canned(Arc<Mutex<Vec<String>>>, Result<Vec<u8>, Unavailable>);

    impl Internet for Canned {
        fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
            self.0.lock().unwrap().push(url.into());
            read(&mut self.1.clone()?.as_slice())
        }
    }

    fn paris() -> Place {
        Place { name: "Paris".into(), point: GeoPoint { latitude: 48.8566, longitude: 2.3522 } }
    }

    #[test]
    fn an_answer_becomes_today_and_the_week() {
        let forecast = read_forecast(ANSWER.as_bytes()).unwrap();
        let today = Today {
            sky: Sky::Cloudy,
            now: Degrees(19),
            feels_like: Degrees(17),
            humidity: Percent::new(64).unwrap(),
            pressure: Hectopascals(1016),
            wind: Wind { speed: KilometresPerHour(12), from: CompassPoint::SouthWest },
        };
        assert_eq!(forecast.today, today);
        assert_eq!(forecast.week.len(), 7);
        let saturday = &forecast.week[1];
        assert_eq!(saturday.date.weekday(), Weekday::Saturday);
        assert_eq!((saturday.sky, saturday.low, saturday.high), (Sky::Rain, Degrees(12), Degrees(17)));
        assert_eq!(saturday.rain_chance, Percent::new(85));
        assert_eq!((saturday.sunrise, saturday.sunset), (TimeOfDay::new(7, 41), TimeOfDay::new(19, 41)));
        assert_eq!(forecast.week[6].rain_chance, None, "null is not known");
    }

    #[test]
    fn a_day_without_sunrise_or_sunset_has_neither() {
        let polar = ANSWER
            .replace(r#""2026-10-01T07:49""#, r#""2026-10-01T00:00""#)
            .replace(r#""2026-10-01T19:30""#, r#""2026-10-01T00:00""#)
            .replace(r#""2026-09-30T07:47""#, "null");
        let forecast = read_forecast(polar.as_bytes()).unwrap();
        assert_eq!((forecast.week[6].sunrise, forecast.week[6].sunset), (None, None), "both at midnight");
        assert_eq!((forecast.week[5].sunrise, forecast.week[5].sunset), (None, TimeOfDay::new(19, 32)));
        assert_eq!(forecast.week[4].sunrise, TimeOfDay::new(7, 46));
        assert_eq!(forecast.week[5].low, Degrees(-3), "rounded away from zero");
        let skies: Vec<Sky> = forecast.week.iter().map(|d| d.sky).collect();
        assert_eq!(skies, [Sky::Cloudy, Sky::Rain, Sky::PartlyCloudy, Sky::Clear, Sky::Fog, Sky::Snow, Sky::Storm]);
    }

    #[test]
    fn an_answer_has_the_hours_ahead() {
        let forecast = read_forecast(ANSWER.as_bytes()).unwrap();
        let hours: Vec<(u8, Sky, Degrees)> =
            forecast.hours.iter().map(|hour| (hour.start.time_of_day.hour(), hour.sky, hour.temperature)).collect();
        assert_eq!(hours, [(17, Sky::Cloudy, Degrees(19)), (18, Sky::Rain, Degrees(17)), (19, Sky::Clear, Degrees(16))]);
        assert_eq!(forecast.hours[0].start.date, Date::new(2026, 9, 25).unwrap());
        let rain: Vec<Option<Percent>> = forecast.hours.iter().map(|hour| hour.rain_chance).collect();
        assert_eq!(rain, [Percent::new(5), Percent::new(70), None]);
        let fallen: Vec<Option<u16>> = forecast.hours.iter().map(|hour| hour.precipitation.map(Millimetres::tenths)).collect();
        assert_eq!(fallen, [Some(0), Some(24), None], "to the nearest tenth");
    }

    #[test]
    fn the_request_names_the_place_and_what_is_wanted() {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let mut source = OpenMeteo::new(Canned(Arc::clone(&asked), Ok(ANSWER.as_bytes().to_vec())));
        source.fetch(&paris()).unwrap();
        let url = asked.lock().unwrap()[0].clone();
        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?latitude=48.8566&longitude=2.3522"), "{url}");
        for wanted in ["current=temperature_2m,weather_code,apparent_temperature,relative_humidity_2m,pressure_msl,wind_speed_10m,wind_direction_10m", "daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,sunrise,sunset", "hourly=temperature_2m,weather_code,precipitation_probability,precipitation&forecast_hours=24", "timezone=auto", "forecast_days=7"] {
            assert!(url.contains(wanted), "{url} lacks {wanted}");
        }
    }

    #[test]
    fn no_internet_is_passed_on_as_it_is() {
        let mut source = OpenMeteo::new(Canned(Arc::default(), Err(Unavailable("no Wi-Fi".into()))));
        assert_eq!(source.fetch(&paris()), Err(Unavailable("no Wi-Fi".into())));
    }

    #[test]
    fn a_truncated_or_strange_answer_is_no_forecast() {
        for cut in [0, 10, ANSWER.len() / 2, ANSWER.len() - 2] {
            assert!(read_forecast(&ANSWER.as_bytes()[..cut]).is_err(), "cut at {cut}");
        }
        let strange_sky = ANSWER.replace("\"weather_code\": 3,", "\"weather_code\": 42,");
        assert_ne!(strange_sky, ANSWER);
        assert!(read_forecast(strange_sky.as_bytes()).is_err());
        let a_day_short = ANSWER.replace("[21.4, 17.0,", "[17.0,");
        assert_ne!(a_day_short, ANSWER);
        assert!(read_forecast(a_day_short.as_bytes()).is_err(), "columns of different lengths");
    }

    #[test]
    fn dates_are_read_strictly() {
        assert_eq!(date_of("2026-09-25"), Ok(Date::new(2026, 9, 25).unwrap()));
        for bad in ["2026-13-01", "2026-09", "yesterday", "2026-09-32"] {
            assert!(date_of(bad).is_err(), "{bad}");
        }
        assert_eq!(time_of("2026-09-25T17:00").map(|time| time.time_of_day), Ok(TimeOfDay::new(17, 0).unwrap()));
        for bad in ["2026-09-25", "2026-09-25T24:00", "2026-09-25T17", "2026-09-25Tnoon:00"] {
            assert!(time_of(bad).is_err(), "{bad}");
        }
    }
}
