use crate::Fault;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPoint {
    pub ssid: String,
    pub rssi_dbm: i8,
    pub channel: u8,
}

pub trait WifiScanner {
    fn scan(&mut self) -> Result<Vec<AccessPoint>, Fault>;
}

/// Joins and leaves a network. Joining blocks until the network gives an address.
pub trait WifiStation {
    fn connect(&mut self, ssid: &str, password: &str) -> Result<(), Fault>;
    fn disconnect(&mut self) -> Result<(), Fault>;
}
