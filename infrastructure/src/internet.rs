//! The Internet, reached through Wi-Fi joined for one request and left right after: the
//! radio is on for seconds an hour, not all night beside a bed.
//!
//! The network is the one in `cute-display/wifi.conf`:
//!
//! ```text
//! ssid = MyNetwork
//! password = secret
//! ```

use domain::weather::Unavailable;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::storage::FileStorage;

use crate::conf_text::ConfText;

pub const WIFI_FILE: &str = "cute-display/wifi.conf";

pub trait Internet: Send {
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Unavailable>;
}

pub struct OnDemandInternet<W, H, S> {
    wifi: W,
    http: H,
    storage: S,
}

impl<W: WifiStation, H: HttpClient, S: FileStorage> OnDemandInternet<W, H, S> {
    pub fn new(wifi: W, http: H, storage: S) -> Self {
        Self { wifi, http, storage }
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
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Unavailable> {
        let (ssid, password) = self.credentials()?;
        let joined = self.wifi.connect(&ssid, &password);
        let body = joined.and_then(|()| self.http.get(url));
        if let Err(fault) = self.wifi.disconnect() {
            log::warn!("wifi: {fault}");
        }
        body.map_err(|fault| Unavailable(fault.to_string()))
    }
}

/// For a device with no way to reach the Internet, and a reason to give.
pub struct NoInternet(pub &'static str);

impl Internet for NoInternet {
    fn get(&mut self, _: &str) -> Result<Vec<u8>, Unavailable> {
        Err(Unavailable(self.0.into()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use hal::Fault;

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
        fn get(&mut self, url: &str) -> Result<Vec<u8>, Fault> {
            self.0.note(format!("get {url}"));
            self.1.clone()
        }
    }

    fn internet(joins: bool, answer: Result<Vec<u8>, Fault>) -> (OnDemandInternet<FakeWifi, FakeHttp, MemoryStorage>, Journal) {
        let journal = Journal::default();
        let storage = MemoryStorage::with(WIFI_FILE, "ssid = Home\npassword = s3cret\n");
        (OnDemandInternet::new(FakeWifi(journal.clone(), joins), FakeHttp(journal.clone(), answer), storage), journal)
    }

    #[test]
    fn a_request_joins_the_network_then_leaves_it() {
        let (mut internet, journal) = internet(true, Ok(b"{}".to_vec()));
        assert_eq!(internet.get("https://x"), Ok(b"{}".to_vec()));
        assert_eq!(journal.entries(), ["connect Home s3cret", "get https://x", "disconnect"]);
    }

    #[test]
    fn the_network_is_left_even_when_the_request_fails() {
        let (mut internet, journal) = internet(true, Err(Fault::new("the server answered 500")));
        assert_eq!(internet.get("https://x"), Err(Unavailable("the server answered 500".into())));
        assert_eq!(journal.entries().last().map(String::as_str), Some("disconnect"));
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
}
