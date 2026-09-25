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
