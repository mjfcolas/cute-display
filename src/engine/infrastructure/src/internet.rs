use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use domain::fetch::Unavailable;
use domain::internet::{BodyReader, Internet};
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::steady::SteadyClock;
use hal::storage::FileStorage;
use hal::udp::UdpClient;
use hal::Fault;

use crate::conf_file;

pub const WIFI_FILE: &str = "cute-display/wifi.conf";
/// A radar asking every fifteen seconds keeps the Wi-Fi, a forecast once an hour does not,
/// and the radio is off the rest of the night.
pub const LINGER: Duration = Duration::from_secs(60);
pub const DATAGRAM_TIMEOUT: Duration = Duration::from_secs(3);

pub trait Datagrams: Send {
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8]) -> Result<usize, Unavailable>;
}

pub struct OnDemandInternet<W, H, U, S> {
    wifi: W,
    http: H,
    udp: U,
    storage: S,
    steady: Box<dyn SteadyClock + Send>,
    link: Link,
}

enum Link {
    Left,
    Joined { last_used: Instant },
}

impl<W: WifiStation, H: HttpClient, U: UdpClient, S: FileStorage> OnDemandInternet<W, H, U, S> {
    pub fn new(wifi: W, http: H, udp: U, storage: S, steady: impl SteadyClock + Send + 'static) -> Self {
        Self { wifi, http, udp, storage, steady: Box::new(steady), link: Link::Left }
    }

    fn used(&mut self) {
        self.link = Link::Joined { last_used: self.steady.now() };
    }

    pub fn release_if_idle(&mut self) {
        let now = self.steady.now();
        if matches!(self.link, Link::Joined { last_used } if now.saturating_duration_since(last_used) >= LINGER) {
            self.leave();
        }
    }

    fn join(&mut self) -> Result<(), Unavailable> {
        if matches!(self.link, Link::Joined { .. }) {
            return Ok(());
        }
        let (ssid, password) = self.credentials()?;
        if let Err(fault) = self.wifi.connect(&ssid, &password) {
            self.leave();
            return Err(Unavailable(fault.to_string()));
        }
        self.used();
        Ok(())
    }

    fn leave(&mut self) {
        self.link = Link::Left;
        if let Err(fault) = self.wifi.disconnect() {
            log::warn!("wifi: {fault}");
        }
    }

    fn credentials(&self) -> Result<(String, String), Unavailable> {
        let missing = || Unavailable(format!("no Wi-Fi: put {WIFI_FILE}"));
        let conf = conf_file::read(&self.storage, WIFI_FILE).map_err(|f| Unavailable(f.to_string()))?.ok_or_else(missing)?;
        let ssid = conf.get("ssid").filter(|s| !s.is_empty()).ok_or_else(missing)?;
        Ok((ssid.to_owned(), conf.get("password").unwrap_or_default().to_owned()))
    }
}

impl<W, H, U, S> Internet for OnDemandInternet<W, H, U, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
    U: UdpClient + Send,
    S: FileStorage + Send,
{
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        self.join()?;
        let mut refused = None;
        let fetched = self.http.fetch(url, &mut |body| {
            read(body).map_err(|unavailable| {
                let fault = Fault::new(&unavailable);
                refused = Some(unavailable);
                fault
            })
        });
        match (fetched, refused) {
            (Ok(()), _) => {
                self.used();
                Ok(())
            }
            // The body arrived but was not what the reader wanted: the network is fine.
            (Err(_), Some(unavailable)) => {
                self.used();
                Err(unavailable)
            }
            // The network may be what failed: start again from scratch next time.
            (Err(fault), None) => {
                self.leave();
                Err(Unavailable(fault.to_string()))
            }
        }
    }
}

impl<W, H, U, S> Datagrams for OnDemandInternet<W, H, U, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
    U: UdpClient + Send,
    S: FileStorage + Send,
{
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8]) -> Result<usize, Unavailable> {
        self.join()?;
        match self.udp.exchange(host, port, request, answer, DATAGRAM_TIMEOUT) {
            Ok(length) => {
                self.used();
                Ok(length)
            }
            Err(fault) => {
                self.leave();
                Err(Unavailable(fault.to_string()))
            }
        }
    }
}

pub struct SharedInternet<W, H, U, S>(Arc<Mutex<OnDemandInternet<W, H, U, S>>>);

impl<W, H, U, S> Clone for SharedInternet<W, H, U, S> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<W: WifiStation, H: HttpClient, U: UdpClient, S: FileStorage> SharedInternet<W, H, U, S> {
    pub fn new(internet: OnDemandInternet<W, H, U, S>) -> Self {
        Self(Arc::new(Mutex::new(internet)))
    }

    /// Does nothing while a request is under way: that request is the opposite of idle.
    pub fn release_if_idle(&self) {
        if let Ok(mut internet) = self.0.try_lock() {
            internet.release_if_idle();
        }
    }
}

impl<W, H, U, S> Internet for SharedInternet<W, H, U, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
    U: UdpClient + Send,
    S: FileStorage + Send,
{
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).fetch(url, read)
    }
}

impl<W, H, U, S> Datagrams for SharedInternet<W, H, U, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
    U: UdpClient + Send,
    S: FileStorage + Send,
{
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8]) -> Result<usize, Unavailable> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).exchange(host, port, request, answer)
    }
}

pub struct NoInternet(pub &'static str);

impl Internet for NoInternet {
    fn fetch(&mut self, _: &str, _: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        Err(Unavailable(self.0.into()))
    }
}

impl Datagrams for NoInternet {
    fn exchange(&mut self, _: &str, _: u16, _: &[u8], _: &mut [u8]) -> Result<usize, Unavailable> {
        Err(Unavailable(self.0.into()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::internet::MAX_HELD_BYTES;
    use hal_testing::steady::FakeSteadyClock;
    use hal_testing::storage::FakeFileStorage;

    use super::*;

    #[derive(Clone, Default)]
    struct Journal(Arc<Mutex<Vec<String>>>);

    impl Journal {
        fn note(&self, what: String) {
            self.0.lock().unwrap().push(what);
        }
        fn entries(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    struct StubWifiStation {
        journal: Journal,
        joins: bool,
    }

    impl WifiStation for StubWifiStation {
        fn connect(&mut self, ssid: &str, password: &str) -> Result<(), Fault> {
            self.journal.note(format!("connect {ssid} {password}"));
            if self.joins { Ok(()) } else { Err(Fault::new("no such network")) }
        }
        fn disconnect(&mut self) -> Result<(), Fault> {
            self.journal.note("disconnect".into());
            Ok(())
        }
    }

    struct StubHttpClient {
        journal: Journal,
        answer: Result<Vec<u8>, Fault>,
    }

    impl HttpClient for StubHttpClient {
        fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
            self.journal.note(format!("get {url}"));
            let body = self.answer.clone()?;
            read(&mut body.as_slice())
        }
    }

    struct StubUdpClient {
        journal: Journal,
        answer: Result<Vec<u8>, Fault>,
    }

    impl UdpClient for StubUdpClient {
        fn exchange(&mut self, host: &str, port: u16, _: &[u8], answer: &mut [u8], _: Duration) -> Result<usize, Fault> {
            self.journal.note(format!("udp {host}:{port}"));
            let datagram = self.answer.clone()?;
            answer[..datagram.len()].copy_from_slice(&datagram);
            Ok(datagram.len())
        }
    }

    type TestInternet = OnDemandInternet<StubWifiStation, StubHttpClient, StubUdpClient, FakeFileStorage>;

    fn internet(joins: bool, answer: Result<Vec<u8>, Fault>) -> (TestInternet, Journal) {
        internet_on(FakeSteadyClock::default(), joins, answer)
    }

    fn internet_on(clock: FakeSteadyClock, joins: bool, answer: Result<Vec<u8>, Fault>) -> (TestInternet, Journal) {
        let journal = Journal::default();
        let storage = FakeFileStorage::with(WIFI_FILE, "ssid = Home\npassword = s3cret\n");
        let http = StubHttpClient { journal: journal.clone(), answer: answer.clone() };
        let udp = StubUdpClient { journal: journal.clone(), answer };
        (OnDemandInternet::new(StubWifiStation { journal: journal.clone(), joins }, http, udp, storage, clock), journal)
    }

    #[test]
    fn requests_close_together_share_one_connection() {
        let (mut internet, journal) = internet(true, Ok(b"{}".to_vec()));
        assert_eq!(internet.get("https://a"), Ok(b"{}".to_vec()));
        assert_eq!(internet.get("https://b"), Ok(b"{}".to_vec()));
        assert_eq!(journal.entries(), ["connect Home s3cret", "get https://a", "get https://b"]);
    }

    #[test]
    fn the_network_is_left_a_minute_after_the_last_request() {
        let clock = FakeSteadyClock::default();
        let (mut internet, journal) = internet_on(clock.clone(), true, Ok(vec![]));
        internet.get("https://a").unwrap();
        clock.advance(LINGER - Duration::from_secs(2));
        internet.release_if_idle();
        assert_eq!(journal.entries().last().map(String::as_str), Some("get https://a"));
        clock.advance(Duration::from_secs(3));
        internet.release_if_idle();
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
    }

    #[test]
    fn a_failed_request_leaves_the_network_to_start_afresh() {
        let (mut internet, journal) = internet(true, Err(Fault::new("the server answered 500")));
        assert_eq!(internet.get("https://x"), Err(Unavailable("the server answered 500".into())));
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
    }

    #[test]
    fn an_answer_the_reader_refuses_keeps_the_network() {
        let (mut internet, journal) = internet(true, Ok(b"garbage".to_vec()));
        let refused = internet.fetch("https://x", &mut |_| Err(Unavailable("unreadable".into())));
        assert_eq!(refused, Err(Unavailable("unreadable".into())));
        assert_eq!(journal.entries(), ["connect Home s3cret", "get https://x"]);
    }

    #[test]
    fn a_network_that_cannot_be_joined_is_not_asked_anything() {
        let (mut internet, journal) = internet(false, Ok(vec![]));
        assert!(internet.get("https://x").is_err());
        assert_eq!(journal.entries(), ["connect Home s3cret", "disconnect"]);
    }

    #[test]
    fn without_wifi_conf_it_says_what_to_do_and_leaves_the_radio_off() {
        let journal = Journal::default();
        let mut internet = OnDemandInternet::new(
            StubWifiStation { journal: journal.clone(), joins: true },
            StubHttpClient { journal: journal.clone(), answer: Ok(vec![]) },
            StubUdpClient { journal: journal.clone(), answer: Ok(vec![]) },
            FakeFileStorage::default(),
            FakeSteadyClock::default(),
        );
        assert_eq!(internet.get("https://x"), Err(Unavailable(format!("no Wi-Fi: put {WIFI_FILE}"))));
        assert!(journal.entries().is_empty());
    }

    #[test]
    fn datagrams_share_the_connection_and_a_lost_one_leaves_it() {
        let (mut internet, journal) = internet(true, Ok(b"pong".to_vec()));
        let mut answer = [0u8; 8];
        assert_eq!(internet.exchange("time", 123, b"ping", &mut answer), Ok(4));
        internet.get("https://a").unwrap();
        assert_eq!(journal.entries(), ["connect Home s3cret", "udp time:123", "get https://a"]);

        let (mut internet, journal) = internet_failing_udp();
        assert!(internet.exchange("time", 123, b"ping", &mut answer).is_err());
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
    }

    fn internet_failing_udp() -> (TestInternet, Journal) {
        let (internet, journal) = internet(true, Ok(vec![]));
        let udp = StubUdpClient { journal: journal.clone(), answer: Err(Fault::new("no answer")) };
        (OnDemandInternet { udp, ..internet }, journal)
    }

    #[test]
    fn an_answer_too_large_to_hold_is_refused() {
        let (mut internet, _) = internet(true, Ok(vec![b'x'; MAX_HELD_BYTES as usize + 1]));
        assert!(internet.get("https://x").is_err());
    }

    #[test]
    fn clones_of_a_shared_internet_use_the_same_connection() {
        let clock = FakeSteadyClock::default();
        let (internet, journal) = internet_on(clock.clone(), true, Ok(vec![]));
        let shared = SharedInternet::new(internet);
        let mut weather = shared.clone();
        let mut radar = shared.clone();
        weather.get("https://weather").unwrap();
        radar.get("https://radar").unwrap();
        assert_eq!(journal.entries(), ["connect Home s3cret", "get https://weather", "get https://radar"]);
        clock.advance(LINGER);
        shared.release_if_idle();
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
    }
}
