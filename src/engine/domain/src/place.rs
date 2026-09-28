//! Where on Earth: a point, and a place a person names.

/// Kilometres, as the device reckons short distances.
const EARTH_RADIUS_KM: f64 = 6371.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

impl GeoPoint {
    /// Kilometres east and north of `origin`. A flat projection around `origin`: off by
    /// well under a percent at the hundred kilometres the device ever looks at.
    pub fn offset_from(self, origin: GeoPoint) -> (f64, f64) {
        let east_degrees = (self.longitude - origin.longitude + 540.0).rem_euclid(360.0) - 180.0;
        let mid_latitude = ((self.latitude + origin.latitude) / 2.0).to_radians();
        let east = east_degrees.to_radians() * mid_latitude.cos() * EARTH_RADIUS_KM;
        let north = (self.latitude - origin.latitude).to_radians() * EARTH_RADIUS_KM;
        (east, north)
    }

    pub fn distance_km(self, other: GeoPoint) -> f64 {
        let (east, north) = self.offset_from(other);
        east.hypot(north)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    /// What the place is called on the glass.
    pub name: String,
    pub point: GeoPoint,
}

pub trait PlaceSource: Send {
    /// `None` when no place has been set.
    fn place(&mut self) -> Option<Place>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARIS: GeoPoint = GeoPoint { latitude: 48.8566, longitude: 2.3522 };
    const ORLY: GeoPoint = GeoPoint { latitude: 48.7233, longitude: 2.3794 };

    #[test]
    fn orly_is_fifteen_kilometres_south_of_paris() {
        let (east, north) = ORLY.offset_from(PARIS);
        assert!((1.9..2.1).contains(&east), "{east}");
        assert!((-14.9..-14.7).contains(&north), "{north}");
        assert!((ORLY.distance_km(PARIS) - PARIS.distance_km(ORLY)).abs() < 1e-9);
    }

    #[test]
    fn offsets_point_the_right_way_even_across_the_date_line() {
        let origin = GeoPoint { latitude: 0.0, longitude: 179.9 };
        let (east, north) = GeoPoint { latitude: -0.1, longitude: -179.9 }.offset_from(origin);
        assert!((22.0..23.0).contains(&east), "east across the date line: {east}");
        assert!(north < 0.0);
    }
}
