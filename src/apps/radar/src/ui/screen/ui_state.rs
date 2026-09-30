#[derive(Clone, Debug, PartialEq)]
pub struct RadarUiState {
    pub title: String,
    pub range_km: u32,
    pub airports: Vec<AirportMark>,
    /// Nearest first.
    pub aircraft: Vec<AircraftMark>,
    pub nearest: Vec<ListedAircraft>,
    pub trouble: Option<String>,
    pub hints: [&'static str; 2],
    pub credit: &'static str,
}

/// Where a mark is from the place, within the range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FromPlace {
    pub east_km: f64,
    pub north_km: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AirportMark {
    pub from_place: FromPlace,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AircraftMark {
    pub from_place: FromPlace,
    pub track_degrees: Option<f32>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedAircraft {
    pub identity: String,
    pub whereabouts: String,
}
