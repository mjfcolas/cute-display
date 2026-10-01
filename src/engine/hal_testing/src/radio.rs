use std::sync::{Arc, Mutex};

use hal::radio::WifiStation;
use hal::Fault;

use crate::shared::lock;

/// Joins any network at once, and keeps whether it is joined and how many times.
#[derive(Clone, Default)]
pub struct StubWifiStation(Arc<Mutex<Station>>);

#[derive(Default)]
struct Station {
    joined: bool,
    joins: usize,
}

impl StubWifiStation {
    pub fn is_joined(&self) -> bool {
        lock(&self.0).joined
    }

    pub fn joins(&self) -> usize {
        lock(&self.0).joins
    }
}

impl WifiStation for StubWifiStation {
    fn connect(&mut self, _: &str, _: &str) -> Result<(), Fault> {
        let mut station = lock(&self.0);
        station.joined = true;
        station.joins += 1;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), Fault> {
        lock(&self.0).joined = false;
        Ok(())
    }
}
