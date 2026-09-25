use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use hal::radio::{AccessPoint, WifiScanner};
use hal::Fault;

use crate::or_fault::OrFault;

pub struct EspWifiScanner {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl EspWifiScanner {
    /// Without NVS: that partition belongs to the stock firmware.
    pub fn new(modem: Modem<'static>, events: EspSystemEventLoop) -> Result<Self, Fault> {
        let wifi = EspWifi::new(modem, events.clone(), None).or_fault("Wi-Fi driver")?;
        let mut wifi = BlockingWifi::wrap(wifi, events).or_fault("Wi-Fi driver")?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration::default())).or_fault("Wi-Fi station mode")?;
        wifi.start().or_fault("starting Wi-Fi")?;
        Ok(Self { wifi })
    }
}

impl WifiScanner for EspWifiScanner {
    fn scan(&mut self) -> Result<Vec<AccessPoint>, Fault> {
        let found = self.wifi.scan().or_fault("Wi-Fi scan")?;
        Ok(found
            .into_iter()
            .map(|ap| AccessPoint { ssid: ap.ssid.to_string(), rssi_dbm: ap.signal_strength, channel: ap.channel })
            .collect())
    }
}
