use std::io::Read;
use std::sync::{Arc, Mutex};

use hal::http::HttpClient;
use hal::Fault;

use crate::shared::lock;

/// Answers the URLs it was given an answer for, by how they start, and keeps the URLs asked;
/// any other is a fault, as an unreachable server is.
#[derive(Clone, Default)]
pub struct StubHttpClient(Arc<Mutex<Web>>);

#[derive(Default)]
struct Web {
    answers: Vec<(String, Vec<u8>)>,
    asked: Vec<String>,
}

impl StubHttpClient {
    pub fn answering(self, url_start: &str, body: impl Into<Vec<u8>>) -> Self {
        self.starts_answering(url_start, body);
        self
    }

    /// For a server that comes back while the test runs.
    pub fn starts_answering(&self, url_start: &str, body: impl Into<Vec<u8>>) {
        lock(&self.0).answers.push((url_start.into(), body.into()));
    }

    pub fn asked(&self) -> Vec<String> {
        lock(&self.0).asked.clone()
    }
}

impl HttpClient for StubHttpClient {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let body = {
            let mut web = lock(&self.0);
            web.asked.push(url.into());
            web.answers.iter().find(|(start, _)| url.starts_with(start.as_str())).map(|(_, body)| body.clone())
        };
        let body = body.ok_or_else(|| Fault::new(format!("{url}: nothing answers")))?;
        read(&mut body.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetch(web: &mut StubHttpClient, url: &str) -> Result<Vec<u8>, Fault> {
        let mut body = Vec::new();
        web.fetch(url, &mut |read| read.read_to_end(&mut body).map(drop).map_err(Fault::new))?;
        Ok(body)
    }

    #[test]
    fn a_url_is_answered_by_the_answer_its_start_was_given_and_kept_as_asked() {
        let mut web = StubHttpClient::default().answering("https://api.open-meteo.com/", "{}");
        assert_eq!(fetch(&mut web, "https://api.open-meteo.com/v1/forecast?latitude=48.85"), Ok(b"{}".to_vec()));
        assert!(fetch(&mut web, "https://opendata.adsb.fi/api/v3").is_err(), "nothing answers it");
        assert_eq!(web.asked(), ["https://api.open-meteo.com/v1/forecast?latitude=48.85", "https://opendata.adsb.fi/api/v3"]);
    }
}
