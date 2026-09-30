//! Aircraft around a point, from adsb.fi's open data (opendata.adsb.fi): free, no key,
//! one request a second, personal and non-commercial use, attribution required. The
//! answer follows ADS-B Exchange's v2 format, which other feeds share.
//!
//! A busy sky is tens of kilobytes of JSON with some forty fields an aircraft, so the
//! answer is read as it arrives and only the nearest aircraft are ever kept.

use std::fmt;
use std::io::{BufReader, Read};

use domain::fetch::Unavailable;
use domain::internet::Internet;
use domain::place::GeoPoint;
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::domain::radar::{AirTrafficSource, Aircraft, Altitude, MAX_AIRCRAFT};

const KM_PER_NAUTICAL_MILE: f64 = 1.852;
const MAX_RADIUS_NM: u32 = 250;

pub struct AdsbFi<I> {
    internet: I,
}

impl<I: Internet> AdsbFi<I> {
    pub fn new(internet: I) -> Self {
        Self { internet }
    }
}

impl<I: Internet> AirTrafficSource for AdsbFi<I> {
    fn nearby(&mut self, center: GeoPoint, radius_km: u32) -> Result<Vec<Aircraft>, Unavailable> {
        let mut aircraft = Vec::new();
        self.internet.fetch(&url(center, radius_km), &mut |body| {
            aircraft = read_nearest(body, center, MAX_AIRCRAFT)?;
            Ok(())
        })?;
        Ok(aircraft)
    }
}

fn url(center: GeoPoint, radius_km: u32) -> String {
    let nautical_miles = (f64::from(radius_km) / KM_PER_NAUTICAL_MILE).ceil().clamp(1.0, f64::from(MAX_RADIUS_NM));
    format!(
        "https://opendata.adsb.fi/api/v3/lat/{:.4}/lon/{:.4}/dist/{nautical_miles}",
        center.latitude, center.longitude
    )
}

fn read_nearest(body: &mut dyn Read, center: GeoPoint, keep: usize) -> Result<Vec<Aircraft>, Unavailable> {
    let mut json = serde_json::Deserializer::from_reader(BufReader::with_capacity(512, body));
    let aircraft = Answer { center, keep }.deserialize(&mut json).map_err(|e| Unavailable(format!("unreadable traffic: {e}")))?;
    json.end().map_err(|e| Unavailable(format!("unreadable traffic: {e}")))?;
    Ok(aircraft)
}

#[derive(Deserialize)]
struct Reported {
    flight: Option<String>,
    #[serde(rename = "r")]
    registration: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    alt_baro: Option<BarometricAltitude>,
    // Read as f64: letting serde produce an f32 here crashes the Xtensa code generator of
    // the `esp` toolchain (LLVM "Cannot select XtensaISD::PCREL_WRAPPER").
    track: Option<f64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BarometricAltitude {
    Feet(f64),
    /// Always `"ground"`.
    Ground(IgnoredAny),
}

impl Reported {
    fn into_aircraft(self) -> Option<Aircraft> {
        let point = GeoPoint { latitude: self.lat?, longitude: self.lon? };
        let trimmed = |text: Option<String>| text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
        let callsign = trimmed(self.flight);
        let registration = trimmed(self.registration);
        let altitude = self.alt_baro.map(|a| match a {
            BarometricAltitude::Feet(feet) => Altitude::Feet(feet.round() as i32),
            BarometricAltitude::Ground(_) => Altitude::Ground,
        });
        Some(Aircraft { callsign, registration, point, altitude, track_degrees: self.track.map(|t| t as f32) })
    }
}

struct Answer {
    center: GeoPoint,
    keep: usize,
}

impl<'de> DeserializeSeed<'de> for Answer {
    type Value = Vec<Aircraft>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for Answer {
    type Value = Vec<Aircraft>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object with an `ac` list")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut aircraft = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "ac" {
                aircraft = Some(map.next_value_seed(Nearest { center: self.center, keep: self.keep })?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        aircraft.ok_or_else(|| serde::de::Error::missing_field("ac"))
    }
}

/// The `ac` list, never held at more than twice `keep`.
struct Nearest {
    center: GeoPoint,
    keep: usize,
}

impl Nearest {
    fn trim(&self, aircraft: &mut Vec<Aircraft>) {
        aircraft.sort_by(|a, b| a.point.distance_km(self.center).total_cmp(&b.point.distance_km(self.center)));
        aircraft.truncate(self.keep);
    }
}

impl<'de> DeserializeSeed<'de> for Nearest {
    type Value = Vec<Aircraft>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Nearest {
    type Value = Vec<Aircraft>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of aircraft")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut aircraft = Vec::with_capacity(self.keep * 2);
        while let Some(reported) = seq.next_element::<Reported>()? {
            aircraft.extend(reported.into_aircraft());
            if aircraft.len() >= self.keep * 2 {
                self.trim(&mut aircraft);
            }
        }
        self.trim(&mut aircraft);
        Ok(aircraft)
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use domain_testing::internet::StubInternet;

    use super::*;

    const NOTRE_DAME: GeoPoint = GeoPoint { latitude: 48.8530, longitude: 2.3499 };

    /// Shaped like a real answer: a padded callsign, one on the ground, one without a
    /// position, and fields the radar never reads.
    const ANSWER: &str = r#"{
        "ac": [
            {"hex": "39856a", "type": "adsb_icao", "flight": "AFR1234 ", "r": "F-GKXA", "t": "A320",
             "alt_baro": 35000, "alt_geom": 35500, "gs": 452.1, "track": 87.3,
             "lat": 48.9, "lon": 2.5, "nic": 8, "rc": 186, "seen_pos": 0.4, "messages": 12345, "seen": 0.1, "rssi": -20.1},
            {"hex": "3c6444", "flight": "        ", "alt_baro": "ground", "lat": 48.85, "lon": 2.35},
            {"hex": "4ca7b3", "flight": "RYR99X  ", "alt_baro": 12000},
            {"hex": "400f01", "flight": "BAW316", "alt_baro": 24000, "track": 310, "lat": 49.2, "lon": 2.0}
        ],
        "msg": "No error", "now": 1790000000123, "total": 4, "ctime": 1790000000123, "ptime": 3
    }"#;

    fn read(body: &str) -> Result<Vec<Aircraft>, Unavailable> {
        read_nearest(&mut body.as_bytes(), NOTRE_DAME, 60)
    }

    #[test]
    fn an_answer_becomes_the_aircraft_with_a_position_nearest_first() {
        let aircraft = read(ANSWER).unwrap();
        let callsigns: Vec<Option<&str>> = aircraft.iter().map(|a| a.callsign.as_deref()).collect();
        assert_eq!(callsigns, [None, Some("AFR1234"), Some("BAW316")]);
        assert_eq!(aircraft[0].altitude, Some(Altitude::Ground));
        assert_eq!(aircraft[1].altitude, Some(Altitude::Feet(35_000)));
        assert_eq!(aircraft[1].track_degrees, Some(87.3));
        assert_eq!(aircraft[1].registration.as_deref(), Some("F-GKXA"));
        assert_eq!(aircraft[0].registration, None);
        assert_eq!(aircraft[2].point, GeoPoint { latitude: 49.2, longitude: 2.0 });
    }

    #[test]
    fn only_the_nearest_are_kept_however_busy_the_sky() {
        let many: Vec<String> = (0..500)
            .map(|n| format!(r#"{{"flight": "F{n}", "lat": {}, "lon": 2.3499}}"#, 48.8530 + f64::from(500 - n) * 0.001))
            .collect();
        let answer = format!(r#"{{"ac": [{}]}}"#, many.join(","));
        let aircraft = read_nearest(&mut answer.as_bytes(), NOTRE_DAME, 10).unwrap();
        assert_eq!(aircraft.len(), 10);
        assert_eq!(aircraft[0].callsign.as_deref(), Some("F499"));
    }

    struct StubTricklingReader<'a>(&'a [u8]);

    impl Read for StubTricklingReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some((&first, rest)) = self.0.split_first() else { return Ok(0) };
            if let Some(slot) = buf.first_mut() {
                *slot = first;
                self.0 = rest;
                return Ok(1);
            }
            Ok(0)
        }
    }

    #[test]
    fn an_answer_arriving_byte_by_byte_reads_the_same() {
        let trickled = read_nearest(&mut StubTricklingReader(ANSWER.as_bytes()), NOTRE_DAME, 60).unwrap();
        assert_eq!(trickled, read(ANSWER).unwrap());
    }

    #[test]
    fn a_truncated_or_strange_answer_is_no_traffic() {
        for cut in [0, 10, ANSWER.len() / 2, ANSWER.len() - 2] {
            let truncated = ANSWER.as_bytes().get(..cut).unwrap();
            assert!(read_nearest(&mut &truncated[..], NOTRE_DAME, 60).is_err(), "cut at {cut}");
        }
        assert!(read(r#"{"msg": "No error"}"#).is_err(), "no `ac` at all");
        assert!(read(r#"{"ac": []} trailing"#).is_err());
        assert_eq!(read(r#"{"ac": []}"#), Ok(vec![]));
    }

    #[test]
    fn the_request_asks_for_the_radius_in_nautical_miles() {
        assert_eq!(url(NOTRE_DAME, 25), "https://opendata.adsb.fi/api/v3/lat/48.8530/lon/2.3499/dist/14");
        assert!(url(NOTRE_DAME, 1000).ends_with("/dist/250"));
        assert!(url(NOTRE_DAME, 0).ends_with("/dist/1"));
    }

    #[test]
    fn nearby_asks_the_feed_and_reads_its_answer() {
        let internet = StubInternet::answering(ANSWER);
        let aircraft = AdsbFi::new(internet.clone()).nearby(NOTRE_DAME, 10).unwrap();
        assert_eq!(aircraft.len(), 3);
        assert!(internet.asked()[0].ends_with("/dist/6"));
    }
}
