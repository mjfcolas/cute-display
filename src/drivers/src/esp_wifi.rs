use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use hal::radio::{AccessPoint, WifiScanner, WifiStation};
use hal::Fault;

use crate::or_fault::OrFault;

/// The ESP32's Wi-Fi, off until something needs it.
pub struct EspWifiRadio {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl EspWifiRadio {
    /// Without NVS: the credentials come from the SD card at every connection.
    pub fn new(modem: Modem<'static>, events: EspSystemEventLoop) -> Result<Self, Fault> {
        let wifi = EspWifi::new(modem, events.clone(), None).or_fault("Wi-Fi driver")?;
        Ok(Self { wifi: BlockingWifi::wrap(wifi, events).or_fault("Wi-Fi driver")? })
    }

    fn start(&mut self, configuration: ClientConfiguration) -> Result<(), Fault> {
        self.wifi.set_configuration(&Configuration::Client(configuration)).or_fault("Wi-Fi station mode")?;
        if !self.wifi.is_started().or_fault("Wi-Fi state")? {
            self.wifi.start().or_fault("starting Wi-Fi")?;
        }
        Ok(())
    }
}

impl WifiScanner for EspWifiRadio {
    fn scan(&mut self) -> Result<Vec<AccessPoint>, Fault> {
        if !self.wifi.is_started().or_fault("Wi-Fi state")? {
            self.start(ClientConfiguration::default())?;
        }
        let found = self.wifi.scan().or_fault("Wi-Fi scan")?;
        Ok(found
            .into_iter()
            .map(|ap| AccessPoint { ssid: ap.ssid.to_string(), rssi_dbm: ap.signal_strength, channel: ap.channel })
            .collect())
    }
}

impl WifiStation for EspWifiRadio {
    fn connect(&mut self, ssid: &str, password: &str) -> Result<(), Fault> {
        let configuration = ClientConfiguration {
            ssid: ssid.try_into().map_err(|_| Fault::new("the Wi-Fi name is longer than 32 bytes"))?,
            password: password.try_into().map_err(|_| Fault::new("the Wi-Fi password is longer than 64 bytes"))?,
            auth_method: if password.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal },
            ..Default::default()
        };
        self.start(configuration)?;
        self.wifi.connect().or_fault(&format!("joining {ssid}"))?;
        self.wifi.wait_netif_up().or_fault("getting an address")
    }

    /// Also turns the radio off.
    fn disconnect(&mut self) -> Result<(), Fault> {
        // Fails when Wi-Fi was never started; stopping ends any association anyway.
        let _ = self.wifi.disconnect();
        self.wifi.stop().or_fault("stopping Wi-Fi")
    }
}
