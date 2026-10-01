use core::time::Duration;
use std::sync::{Arc, Mutex};

use hal::udp::UdpClient;
use hal::Fault;

use crate::shared::lock;

/// Gives every request the same answer, or none, and keeps the hosts asked.
#[derive(Clone)]
pub struct StubUdpClient(Arc<Mutex<Exchanges>>);

struct Exchanges {
    answer: Option<Vec<u8>>,
    asked: Vec<String>,
}

impl StubUdpClient {
    /// A server that answers `datagram`, an NTP one `infrastructure::ntp::server_answer`.
    pub fn answering(datagram: impl Into<Vec<u8>>) -> Self {
        Self::new(Some(datagram.into()))
    }

    /// Nothing ever answers.
    pub fn silent() -> Self {
        Self::new(None)
    }

    fn new(answer: Option<Vec<u8>>) -> Self {
        Self(Arc::new(Mutex::new(Exchanges { answer, asked: Vec::new() })))
    }

    pub fn asked(&self) -> Vec<String> {
        lock(&self.0).asked.clone()
    }
}

impl UdpClient for StubUdpClient {
    fn exchange(&mut self, host: &str, port: u16, _: &[u8], answer: &mut [u8], _: Duration) -> Result<usize, Fault> {
        let mut exchanges = lock(&self.0);
        exchanges.asked.push(format!("{host}:{port}"));
        let datagram = exchanges.answer.clone().ok_or_else(|| Fault::new(format!("{host}: no answer")))?;
        let room = answer.get_mut(..datagram.len()).ok_or_else(|| Fault::new("the answer is larger than the room for it"))?;
        room.copy_from_slice(&datagram);
        Ok(datagram.len())
    }
}
