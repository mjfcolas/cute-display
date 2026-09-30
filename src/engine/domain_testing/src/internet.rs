use std::sync::{Arc, Mutex};

use domain::fetch::Unavailable;
use domain::internet::{BodyReader, Internet};

use crate::shared::lock;

/// Gives every request the same answer, and keeps the URLs asked.
#[derive(Clone)]
pub struct StubInternet {
    answer: Result<Vec<u8>, Unavailable>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl StubInternet {
    pub fn answering(body: impl Into<Vec<u8>>) -> Self {
        Self { answer: Ok(body.into()), asked: Arc::default() }
    }

    pub fn unavailable(reason: &str) -> Self {
        Self { answer: Err(Unavailable(reason.into())), asked: Arc::default() }
    }

    pub fn asked(&self) -> Vec<String> {
        lock(&self.asked).clone()
    }
}

impl Internet for StubInternet {
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        lock(&self.asked).push(url.into());
        read(&mut self.answer.clone()?.as_slice())
    }
}
