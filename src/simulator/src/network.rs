use core::time::Duration;

use drivers::udp_socket::StdUdpClient;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::udp::UdpClient;
use hal::Fault;

use crate::ntp_server::SimulatedNtpServer;
use crate::recordings::{RecordedWeb, RecordingWeb};

/// The computer is already on a network; offline, it joins none.
pub struct HostWifi {
    reach: Reach,
}

enum Reach {
    Online,
    Offline,
}

impl HostWifi {
    pub fn online() -> Self {
        Self { reach: Reach::Online }
    }

    pub fn offline() -> Self {
        Self { reach: Reach::Offline }
    }
}

impl WifiStation for HostWifi {
    fn connect(&mut self, _ssid: &str, _password: &str) -> Result<(), Fault> {
        match self.reach {
            Reach::Online => Ok(()),
            Reach::Offline => Err(Fault::new("the simulator is offline")),
        }
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

pub enum Web {
    TheComputers(HostHttpClient),
    Recorded(RecordedWeb),
    Recording(RecordingWeb),
}

impl HttpClient for Web {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn std::io::Read) -> Result<(), Fault>) -> Result<(), Fault> {
        match self {
            Self::TheComputers(web) => web.fetch(url, read),
            Self::Recorded(web) => web.fetch(url, read),
            Self::Recording(web) => web.fetch(url, read),
        }
    }
}

pub enum TimeServer {
    TheComputers(StdUdpClient),
    Simulated(SimulatedNtpServer),
}

impl UdpClient for TimeServer {
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8], timeout: Duration) -> Result<usize, Fault> {
        match self {
            Self::TheComputers(udp) => udp.exchange(host, port, request, answer, timeout),
            Self::Simulated(server) => server.exchange(host, port, request, answer, timeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_no_network_is_joined() {
        assert!(HostWifi::online().connect("Home", "s3cret").is_ok());
        assert!(HostWifi::offline().connect("Home", "s3cret").is_err());
    }
}
