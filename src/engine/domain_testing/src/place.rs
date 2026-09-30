use std::sync::{Arc, Mutex};

use domain::place::{GeoPoint, Place, PlaceSource};

use crate::shared::lock;

pub fn paris() -> Place {
    Place { name: "Paris".into(), point: GeoPoint { latitude: 48.85, longitude: 2.35 } }
}

/// Where the device is, as the test says, which it may change.
#[derive(Clone)]
pub struct StubPlace(Arc<Mutex<Option<Place>>>);

impl StubPlace {
    pub fn at(place: Place) -> Self {
        Self(Arc::new(Mutex::new(Some(place))))
    }

    /// No place was ever set.
    pub fn nowhere() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    pub fn move_to(&self, place: Place) {
        *lock(&self.0) = Some(place);
    }
}

impl PlaceSource for StubPlace {
    fn place(&mut self) -> Option<Place> {
        lock(&self.0).clone()
    }
}
