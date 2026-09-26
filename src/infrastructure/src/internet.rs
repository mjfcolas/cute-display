//! The Internet, reached through Wi-Fi joined on the first request and left once nothing
//! has asked for a minute: a radar asking every fifteen seconds keeps it, a forecast once
//! an hour does not, and the radio is off the rest of the night.
//!
//! The network is the one in `cute-display/wifi.conf`:
//!
//! ```text
//! ssid = MyNetwork
//! password = secret
//! ```

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use domain::fetch::Unavailable;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::Fault;

use crate::conf_text::ConfText;

pub const WIFI_FILE: &str = "cute-display/wifi.conf";
pub const LINGER: Duration = Duration::from_secs(60);
/// What a caller may hold whole with `get`; anything larger has to be streamed.
pub const MAX_HELD_BYTES: u64 = 32 * 1024;

pub type BodyReader<'a> = dyn FnMut(&mut dyn Read) -> Result<(), Unavailable> + 'a;

pub trait Internet: Send {
    /// Hands the body to `read` as it arrives.
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable>;

    /// The whole body, for answers small enough to hold.
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Unavailable> {
        let mut body = Vec::new();
        self.fetch(url, &mut |reader| {
            reader.take(MAX_HELD_BYTES + 1).read_to_end(&mut body).map_err(|e| Unavailable(e.to_string()))?;
            if body.len() as u64 > MAX_HELD_BYTES {
                return Err(Unavailable(format!("the answer is larger than {MAX_HELD_BYTES} bytes")));
            }
            Ok(())
        })?;
        Ok(body)
    }
}

pub struct OnDemandInternet<W, H, S> {
    wifi: W,
    http: H,
    storage: S,
    link: Link,
}

enum Link {
    Left,
    Joined { last_used: Instant },
}

impl<W: WifiStation, H: HttpClient, S: FileStorage> OnDemandInternet<W, H, S> {
    pub fn new(wifi: W, http: H, storage: S) -> Self {
        Self { wifi, http, storage, link: Link::Left }
    }

    /// Leaves the network once nothing has asked for [`LINGER`].
    pub fn release_if_idle(&mut self, now: Instant) {
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
        self.link = Link::Joined { last_used: Instant::now() };
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
        let conf = ConfText::read(&self.storage, WIFI_FILE).map_err(|f| Unavailable(f.to_string()))?.ok_or_else(missing)?;
        let ssid = conf.get("ssid").filter(|s| !s.is_empty()).ok_or_else(missing)?;
        Ok((ssid.to_owned(), conf.get("password").unwrap_or_default().to_owned()))
    }
}

impl<W, H, S> Internet for OnDemandInternet<W, H, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
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
                self.link = Link::Joined { last_used: Instant::now() };
                Ok(())
            }
            // The body arrived but was not what the reader wanted: the network is fine.
            (Err(_), Some(unavailable)) => {
                self.link = Link::Joined { last_used: Instant::now() };
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

/// One way to the Internet, shared by everything that fetches; requests take turns.
pub struct SharedInternet<W, H, S>(Arc<Mutex<OnDemandInternet<W, H, S>>>);

impl<W, H, S> Clone for SharedInternet<W, H, S> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<W: WifiStation, H: HttpClient, S: FileStorage> SharedInternet<W, H, S> {
    pub fn new(internet: OnDemandInternet<W, H, S>) -> Self {
        Self(Arc::new(Mutex::new(internet)))
    }

    /// Does nothing while a request is under way: that request is the opposite of idle.
    pub fn release_if_idle(&self, now: Instant) {
        if let Ok(mut internet) = self.0.try_lock() {
            internet.release_if_idle(now);
        }
    }
}

impl<W, H, S> Internet for SharedInternet<W, H, S>
where
    W: WifiStation + Send,
    H: HttpClient + Send,
    S: FileStorage + Send,
{
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).fetch(url, read)
    }
}

/// For a device with no way to reach the Internet, and a reason to give.
pub struct NoInternet(pub &'static str);

impl Internet for NoInternet {
    fn fetch(&mut self, _: &str, _: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        Err(Unavailable(self.0.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    /// Remembers what it was asked to do, in order.
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

    struct FakeWifi(Journal, bool);

    impl WifiStation for FakeWifi {
        fn connect(&mut self, ssid: &str, password: &str) -> Result<(), Fault> {
            self.0.note(format!("connect {ssid} {password}"));
            if self.1 { Ok(()) } else { Err(Fault::new("no such network")) }
        }
        fn disconnect(&mut self) -> Result<(), Fault> {
            self.0.note("disconnect".into());
            Ok(())
        }
    }

    struct FakeHttp(Journal, Result<Vec<u8>, Fault>);

    impl HttpClient for FakeHttp {
        fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
            self.0.note(format!("get {url}"));
            let body = self.1.clone()?;
            read(&mut body.as_slice())
        }
    }

    type TestInternet = OnDemandInternet<FakeWifi, FakeHttp, MemoryStorage>;

    fn internet(joins: bool, answer: Result<Vec<u8>, Fault>) -> (TestInternet, Journal) {
        let journal = Journal::default();
        let storage = MemoryStorage::with(WIFI_FILE, "ssid = Home\npassword = s3cret\n");
        (OnDemandInternet::new(FakeWifi(journal.clone(), joins), FakeHttp(journal.clone(), answer), storage), journal)
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
        let (mut internet, journal) = internet(true, Ok(vec![]));
        internet.get("https://a").unwrap();
        let used = Instant::now();
        internet.release_if_idle(used + LINGER - Duration::from_secs(2));
        assert_eq!(journal.entries().last().map(String::as_str), Some("get https://a"));
        internet.release_if_idle(used + LINGER + Duration::from_secs(1));
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
        let mut internet =
            OnDemandInternet::new(FakeWifi(journal.clone(), true), FakeHttp(journal.clone(), Ok(vec![])), MemoryStorage::default());
        assert_eq!(internet.get("https://x"), Err(Unavailable(format!("no Wi-Fi: put {WIFI_FILE}"))));
        assert!(journal.entries().is_empty());
    }

    #[test]
    fn an_answer_too_large_to_hold_is_refused() {
        let (mut internet, _) = internet(true, Ok(vec![b'x'; MAX_HELD_BYTES as usize + 1]));
        assert!(internet.get("https://x").is_err());
    }

    #[test]
    fn clones_of_a_shared_internet_use_the_same_connection() {
        let (internet, journal) = internet(true, Ok(vec![]));
        let shared = SharedInternet::new(internet);
        let mut weather = shared.clone();
        let mut radar = shared.clone();
        weather.get("https://weather").unwrap();
        radar.get("https://radar").unwrap();
        assert_eq!(journal.entries(), ["connect Home s3cret", "get https://weather", "get https://radar"]);
        shared.release_if_idle(Instant::now() + LINGER + Duration::from_secs(1));
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
    }
}
