use domain::place::CompassPoint;
use forecast::{Degrees, Millimetres, Sky};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherUiState {
    pub title: String,
    pub page: ShownPage,
    pub status: String,
}

/// `None` until the first forecast arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShownPage {
    Today(Option<Box<TodayPage>>),
    Week(Option<Vec<WeekDay>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodayPage {
    pub sky: Sky,
    pub temperature: Degrees,
    pub feels_like: Degrees,
    pub day: Option<String>,
    pub humidity: String,
    pub pressure: String,
    pub wind: String,
    /// `None` when there is no wind, so no direction.
    pub wind_from: Option<CompassPoint>,
    pub sun: String,
    pub hours: Vec<HourColumn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HourColumn {
    pub hour: String,
    pub sky: Sky,
    pub temperature: String,
    pub likely_rain: Option<String>,
    pub fallen: Option<Millimetres>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekDay {
    pub name: &'static str,
    pub sky: Sky,
    pub range: String,
    pub rain: Option<String>,
}
