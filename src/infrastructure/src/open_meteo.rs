//! Forecasts from Open-Meteo (open-meteo.com): free, no key, current weather and seven
//! days in one call, with dates already in the place's own time zone.

use domain::calendar::Date;
use domain::fetch::Unavailable;
use domain::place::Place;
use domain::weather::{DayForecast, Degrees, Forecast, ForecastSource, Sky, Today};
use serde::Deserialize;

use crate::internet::Internet;

const DAYS: usize = 7;

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
         &current=temperature_2m,weather_code\
         &daily=weather_code,temperature_2m_max,temperature_2m_min\
         &timezone=auto&forecast_days={DAYS}",
        place.point.latitude, place.point.longitude
    )
}

#[derive(Deserialize)]
struct Answer {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: u16,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<u16>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

fn read_forecast(body: &[u8]) -> Result<Forecast, Unavailable> {
    let answer: Answer = serde_json::from_slice(body).map_err(|e| Unavailable(format!("unreadable forecast: {e}")))?;
    let daily = answer.daily;
    let week = daily
        .time
        .iter()
        .zip(&daily.weather_code)
        .zip(daily.temperature_2m_min.iter().zip(&daily.temperature_2m_max))
        .map(|((date, &code), (&low, &high))| {
            Ok(DayForecast { date: date_of(date)?, sky: sky_of(code)?, low: degrees(low), high: degrees(high) })
        })
        .collect::<Result<Vec<_>, Unavailable>>()?;
    let first = week.first().ok_or_else(|| Unavailable("the forecast has no days".into()))?;
    let today = Today {
        sky: sky_of(answer.current.weather_code)?,
        now: degrees(answer.current.temperature_2m),
        low: first.low,
        high: first.high,
    };
    Ok(Forecast { today, week })
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
    use domain::place::GeoPoint;

    use super::*;
    use crate::internet::BodyReader;

    /// Shaped like a real answer, trimmed of the fields not asked for.
    const ANSWER: &str = r#"{
        "latitude": 48.86, "longitude": 2.3399997, "timezone": "Europe/Paris",
        "current_units": {"time": "iso8601", "temperature_2m": "°C", "weather_code": "wmo code"},
        "current": {"time": "2026-09-25T17:45", "interval": 900, "temperature_2m": 18.6, "weather_code": 3},
        "daily_units": {"time": "iso8601", "weather_code": "wmo code"},
        "daily": {
            "time": ["2026-09-25", "2026-09-26", "2026-09-27", "2026-09-28", "2026-09-29", "2026-09-30", "2026-10-01"],
            "weather_code": [3, 61, 2, 0, 45, 71, 95],
            "temperature_2m_max": [21.4, 17.0, 19.2, 22.5, 16.1, 3.4, 18.0],
            "temperature_2m_min": [11.6, 12.3, 9.8, 10.1, 8.0, -2.5, 11.9]
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
        assert_eq!(forecast.today, Today { sky: Sky::Cloudy, now: Degrees(19), low: Degrees(12), high: Degrees(21) });
        assert_eq!(forecast.week.len(), 7);
        let saturday = &forecast.week[1];
        assert_eq!(saturday.date.weekday(), Weekday::Saturday);
        assert_eq!((saturday.sky, saturday.low, saturday.high), (Sky::Rain, Degrees(12), Degrees(17)));
        assert_eq!(forecast.week[5].low, Degrees(-3), "rounded away from zero");
        let skies: Vec<Sky> = forecast.week.iter().map(|d| d.sky).collect();
        assert_eq!(skies, [Sky::Cloudy, Sky::Rain, Sky::PartlyCloudy, Sky::Clear, Sky::Fog, Sky::Snow, Sky::Storm]);
    }

    #[test]
    fn the_request_names_the_place_and_what_is_wanted() {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let mut source = OpenMeteo::new(Canned(Arc::clone(&asked), Ok(ANSWER.as_bytes().to_vec())));
        source.fetch(&paris()).unwrap();
        let url = asked.lock().unwrap()[0].clone();
        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?latitude=48.8566&longitude=2.3522"), "{url}");
        for wanted in ["current=temperature_2m,weather_code", "daily=weather_code,temperature_2m_max,temperature_2m_min", "timezone=auto", "forecast_days=7"] {
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
        let strange_sky = ANSWER.replace("\"weather_code\": 3}", "\"weather_code\": 42}");
        assert_ne!(strange_sky, ANSWER);
        assert!(read_forecast(strange_sky.as_bytes()).is_err());
    }

    #[test]
    fn dates_are_read_strictly() {
        assert_eq!(date_of("2026-09-25"), Ok(Date::new(2026, 9, 25).unwrap()));
        for bad in ["2026-13-01", "2026-09", "yesterday", "2026-09-32"] {
            assert!(date_of(bad).is_err(), "{bad}");
        }
    }
}
