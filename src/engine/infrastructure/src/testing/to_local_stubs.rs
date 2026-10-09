//! The test image's network: it asks stubs on the local network for the web and the time,
//! at the address in its `e2e.conf`. Like the other conf files, the file is read at each
//! use.

use core::time::Duration;
use std::io::Read;

use hal::http::HttpClient;
use hal::storage::FileStorage;
use hal::udp::UdpClient;
use hal::Fault;

use crate::conf_file;
use crate::ntp;

pub const STUBS_FILE: &str = "e2e.conf";
const ADDRESS: &str = "address";
const HTTPS_PORT: &str = "https_port";
const NTP_PORT: &str = "ntp_port";

#[derive(Debug, PartialEq, Eq)]
struct LocalStubs {
    address: String,
    https_port: u16,
    ntp_port: u16,
}

impl LocalStubs {
    fn read(card: &Result<impl FileStorage, Fault>) -> Result<Self, Fault> {
        let card = card.as_ref().map_err(|fault| Fault::new(format!("no stubs: no SD card ({fault})")))?;
        let conf = conf_file::read(card, STUBS_FILE)?.ok_or_else(|| Fault::new(format!("no stubs: put {STUBS_FILE}")))?;
        let address = conf.get(ADDRESS).filter(|address| !address.is_empty());
        let port = |key: &str| conf.get(key).and_then(|port| port.parse::<u16>().ok());
        match (address, port(HTTPS_PORT), port(NTP_PORT)) {
            (Some(address), Some(https_port), Some(ntp_port)) => Ok(Self { address: address.to_owned(), https_port, ntp_port }),
            _ => Err(Fault::new(format!("{STUBS_FILE} needs {ADDRESS}, {HTTPS_PORT} and {NTP_PORT}"))),
        }
    }

    /// The stub answers a URL under its host: `https://api.open-meteo.com/v1/forecast?…` is
    /// asked as `https://<address>:<https_port>/api.open-meteo.com/v1/forecast?…`.
    fn url_for(&self, url: &str) -> Result<String, Fault> {
        let below_scheme = url
            .strip_prefix("https://")
            .filter(|rest| !rest.is_empty())
            .ok_or_else(|| Fault::new(format!("{url}: not a web address")))?;
        Ok(format!("https://{}:{}/{below_scheme}", self.address, self.https_port))
    }
}

pub struct RedirectedHttp<H, S> {
    http: H,
    card: Result<S, Fault>,
}

impl<H: HttpClient, S: FileStorage> RedirectedHttp<H, S> {
    pub fn new(http: H, card: Result<S, Fault>) -> Self {
        Self { http, card }
    }
}

impl<H: HttpClient, S: FileStorage> HttpClient for RedirectedHttp<H, S> {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let redirected = LocalStubs::read(&self.card)?.url_for(url)?;
        self.http.fetch(&redirected, read)
    }
}

pub struct RedirectedUdp<U, S> {
    udp: U,
    card: Result<S, Fault>,
}

impl<U: UdpClient, S: FileStorage> RedirectedUdp<U, S> {
    pub fn new(udp: U, card: Result<S, Fault>) -> Self {
        Self { udp, card }
    }
}

impl<U: UdpClient, S: FileStorage> UdpClient for RedirectedUdp<U, S> {
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8], timeout: Duration) -> Result<usize, Fault> {
        // Only NTP uses UDP.
        if port != ntp::PORT {
            return Err(Fault::new(format!("{host}:{port}: no stub answers but NTP")));
        }
        let stubs = LocalStubs::read(&self.card)?;
        self.udp.exchange(&stubs.address, stubs.ntp_port, request, answer, timeout)
    }
}

#[cfg(test)]
mod tests {
    use hal_testing::http::StubHttpClient;
    use hal_testing::storage::FakeFileStorage;
    use hal_testing::udp::StubUdpClient;

    use super::*;

    const STUBS: &str = "address = 192.168.1.10\nhttps_port = 8443\nntp_port = 8123\n";
    const FORECAST: &str = "https://api.open-meteo.com/v1/forecast?latitude=48.8530&longitude=2.3499";
    const TIMEOUT: Duration = Duration::from_secs(1);

    fn fetch(http: &mut impl HttpClient, url: &str) -> Result<Vec<u8>, Fault> {
        let mut body = Vec::new();
        http.fetch(url, &mut |read| read.read_to_end(&mut body).map(drop).map_err(Fault::new))?;
        Ok(body)
    }

    #[test]
    fn a_url_is_asked_of_the_stubs_under_its_host() {
        let web = StubHttpClient::default().answering("https://192.168.1.10:8443/api.open-meteo.com/", "{}");
        let mut http = RedirectedHttp::new(web.clone(), Ok(FakeFileStorage::with(STUBS_FILE, STUBS)));
        assert_eq!(fetch(&mut http, FORECAST), Ok(b"{}".to_vec()));
        assert_eq!(web.asked(), ["https://192.168.1.10:8443/api.open-meteo.com/v1/forecast?latitude=48.8530&longitude=2.3499"]);
    }

    #[test]
    fn the_stubs_address_is_read_at_every_use() {
        let web = StubHttpClient::default();
        let card = FakeFileStorage::with(STUBS_FILE, STUBS);
        let mut http = RedirectedHttp::new(web.clone(), Ok(card.clone()));
        let _ = fetch(&mut http, "https://a/");
        card.put(STUBS_FILE, "address = 10.0.0.2\nhttps_port = 443\nntp_port = 123\n");
        let _ = fetch(&mut http, "https://a/");
        assert_eq!(web.asked(), ["https://192.168.1.10:8443/a/", "https://10.0.0.2:443/a/"]);
    }

    #[test]
    fn without_a_whole_conf_nothing_is_asked_and_the_fault_says_what_to_put() {
        for (card, said) in [
            (FakeFileStorage::default(), "no stubs: put e2e.conf"),
            (FakeFileStorage::with(STUBS_FILE, "address = 192.168.1.10\nhttps_port = 8443\n"), "e2e.conf needs address, https_port and ntp_port"),
            (FakeFileStorage::with(STUBS_FILE, "address = 192.168.1.10\nhttps_port = high\nntp_port = 8123\n"), "e2e.conf needs address, https_port and ntp_port"),
            (FakeFileStorage::with(STUBS_FILE, "address =\nhttps_port = 8443\nntp_port = 8123\n"), "e2e.conf needs address, https_port and ntp_port"),
        ] {
            let web = StubHttpClient::default().answering("https://", "{}");
            let udp = StubUdpClient::answering(vec![0; ntp::PACKET_BYTES]);
            let fault = fetch(&mut RedirectedHttp::new(web.clone(), Ok(card.clone())), FORECAST).unwrap_err();
            assert_eq!(fault.reason(), said);
            let mut answer = [0; ntp::PACKET_BYTES];
            assert!(RedirectedUdp::new(udp.clone(), Ok(card)).exchange(ntp::SERVER, ntp::PORT, &[], &mut answer, TIMEOUT).is_err());
            assert!(web.asked().is_empty());
            assert!(udp.asked().is_empty());
        }
    }

    #[test]
    fn without_the_card_nothing_is_asked_and_the_fault_says_why() {
        let web = StubHttpClient::default().answering("https://", "{}");
        let mut http = RedirectedHttp::new(web.clone(), Err::<FakeFileStorage, _>(Fault::new("not mounted")));
        assert_eq!(fetch(&mut http, FORECAST).unwrap_err().reason(), "no stubs: no SD card (not mounted)");
        let udp = StubUdpClient::answering(vec![0; ntp::PACKET_BYTES]);
        let mut redirected = RedirectedUdp::new(udp.clone(), Err::<FakeFileStorage, _>(Fault::new("not mounted")));
        let mut answer = [0; ntp::PACKET_BYTES];
        assert!(redirected.exchange(ntp::SERVER, ntp::PORT, &[], &mut answer, TIMEOUT).is_err());
        assert!(web.asked().is_empty());
        assert!(udp.asked().is_empty());
    }

    #[test]
    fn what_is_not_https_is_refused() {
        let web = StubHttpClient::default().answering("https://", "{}");
        let mut http = RedirectedHttp::new(web.clone(), Ok(FakeFileStorage::with(STUBS_FILE, STUBS)));
        for not_web in ["http://a/b", "ftp://a/b", "api.open-meteo.com/v1", "https://"] {
            assert!(fetch(&mut http, not_web).is_err(), "{not_web}");
        }
        assert!(web.asked().is_empty());
    }

    #[test]
    fn the_time_is_asked_of_the_stubs_ntp_port_and_nothing_else_goes_out() {
        let udp = StubUdpClient::answering(vec![7; ntp::PACKET_BYTES]);
        let mut redirected = RedirectedUdp::new(udp.clone(), Ok(FakeFileStorage::with(STUBS_FILE, STUBS)));
        let mut answer = [0; ntp::PACKET_BYTES];
        assert_eq!(redirected.exchange(ntp::SERVER, ntp::PORT, &[], &mut answer, TIMEOUT), Ok(ntp::PACKET_BYTES));
        assert!(redirected.exchange("dns.example", 53, &[], &mut answer, TIMEOUT).is_err());
        assert_eq!(udp.asked(), ["192.168.1.10:8123"]);
    }
}
