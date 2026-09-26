//! The Internet through the computer's own connection.

use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::Fault;

/// Joining is a no-op: the computer is already on a network.
pub struct HostWifi;

impl WifiStation for HostWifi {
    fn connect(&mut self, _ssid: &str, _password: &str) -> Result<(), Fault> {
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}

pub struct HostHttpClient {
    agent: ureq::Agent,
}

impl Default for HostHttpClient {
    fn default() -> Self {
        Self { agent: ureq::Agent::new_with_defaults() }
    }
}

impl HttpClient for HostHttpClient {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn std::io::Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let response = self.agent.get(url).call().map_err(|e| Fault::new(format!("{url}: {e}")))?;
        read(&mut response.into_body().into_reader())
    }
}
