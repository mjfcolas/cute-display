use core::time::Duration;
use std::sync::{Arc, Mutex};

use hal::clock::DateTime;
use hal::udp::UdpClient;
use hal::Fault;

use crate::shared::lock;

/// From 1900-01-01, where NTP counts from, to 1970-01-01.
const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
const NTP_PACKET_BYTES: usize = 48;
/// Leap indicator 0, version 4, mode 4: a server.
const NTP_SERVER_HEADER: u8 = 0b00_100_100;
const NTP_STRATUM: u8 = 2;
const NTP_TRANSMIT_TIMESTAMP: usize = 40;

/// Gives every request the same answer, or none, and keeps the hosts asked.
#[derive(Clone)]
pub struct StubUdpClient(Arc<Mutex<Exchanges>>);

struct Exchanges {
    answer: Option<Vec<u8>>,
    asked: Vec<String>,
}

impl StubUdpClient {
    /// An NTP server that says it is `time`, UTC.
    pub fn ntp_at(time: DateTime) -> Self {
        let mut packet = vec![0; NTP_PACKET_BYTES];
        if let Some(header) = packet.get_mut(..2) {
            header.copy_from_slice(&[NTP_SERVER_HEADER, NTP_STRATUM]);
        }
        let ntp_seconds = u32::try_from(time.unix_seconds() + NTP_TO_UNIX_SECONDS).unwrap_or_default();
        if let Some(timestamp) = packet.get_mut(NTP_TRANSMIT_TIMESTAMP..NTP_TRANSMIT_TIMESTAMP + 4) {
            timestamp.copy_from_slice(&ntp_seconds.to_be_bytes());
        }
        Self::new(Some(packet))
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
