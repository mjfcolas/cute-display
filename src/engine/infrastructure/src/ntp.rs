//! SNTP, RFC 4330.

use domain::clock::TimeSource;
use domain::fetch::Unavailable;
use domain::time::UtcTime;
use hal::clock::DateTime;

use crate::internet::Datagrams;

pub const SERVER: &str = "pool.ntp.org";
const PORT: u16 = 123;
pub const PACKET_BYTES: usize = 48;
/// Leap indicator 0, version 4, mode 3: a client.
const CLIENT_REQUEST: u8 = 0b00_100_011;
/// Leap indicator 0, version 4, mode 4: a server.
const SERVER_HEADER: u8 = 0b00_100_100;
const SERVER_STRATUM: u8 = 2;
const SERVER_MODE: u8 = 4;
const LEAP_UNSYNCHRONISED: u8 = 3;
const TRANSMIT_TIMESTAMP: usize = 40;
/// From 1900-01-01, where NTP counts from, to 1970-01-01.
const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
/// NTP seconds wrap in 2036; below this, they count from then.
const ERA_WRAP: u32 = 1 << 31;
/// Of the fraction of a second, which counts in 2^-32 s.
const HALF_A_SECOND: u32 = 1 << 31;

pub struct NtpServer<D> {
    network: D,
}

impl<D: Datagrams> NtpServer<D> {
    pub fn new(network: D) -> Self {
        Self { network }
    }
}

impl<D: Datagrams> TimeSource for NtpServer<D> {
    fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
        let mut request = [0u8; PACKET_BYTES];
        request[0] = CLIENT_REQUEST;
        // Room for the extensions and authentication some servers add.
        let mut answer = [0u8; 2 * PACKET_BYTES];
        let length = self.network.exchange(SERVER, PORT, &request, &mut answer)?;
        decode(answer.get(..length).unwrap_or_default())
    }
}

/// What a server answers when it is `time`, on the second: for a server played elsewhere
/// than on the Internet.
pub fn server_answer(time: DateTime) -> [u8; PACKET_BYTES] {
    let ntp_seconds = u32::try_from(time.unix_seconds().saturating_add(NTP_TO_UNIX_SECONDS).rem_euclid(1 << 32)).unwrap_or_default();
    let mut packet = [0; PACKET_BYTES];
    for (byte, value) in packet.iter_mut().zip([SERVER_HEADER, SERVER_STRATUM]) {
        *byte = value;
    }
    for (byte, value) in packet.iter_mut().skip(TRANSMIT_TIMESTAMP).zip(ntp_seconds.to_be_bytes()) {
        *byte = value;
    }
    packet
}

fn decode(answer: &[u8]) -> Result<UtcTime, Unavailable> {
    let refused = |why: &str| Unavailable(format!("{SERVER}: {why}"));
    let (Some(&header), Some(&stratum), Some(timestamp)) = (answer.first(), answer.get(1), answer.get(TRANSMIT_TIMESTAMP..PACKET_BYTES))
    else {
        return Err(refused("the answer is too short"));
    };
    if header & 0b111 != SERVER_MODE {
        return Err(refused("not an answer from a server"));
    }
    if header >> 6 == LEAP_UNSYNCHRONISED || !(1..=15).contains(&stratum) {
        return Err(refused("the server does not know the time"));
    }
    let word = |bytes: &[u8]| bytes.try_into().map(u32::from_be_bytes).unwrap_or(0);
    let (seconds, fraction) = (word(timestamp.get(..4).unwrap_or_default()), word(timestamp.get(4..).unwrap_or_default()));
    let era = if seconds < ERA_WRAP { 1i64 << 32 } else { 0 };
    let rounded = i64::from(fraction >= HALF_A_SECOND);
    Ok(UtcTime::from_unix_seconds(i64::from(seconds) + era - NTP_TO_UNIX_SECONDS + rounded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(header: u8, stratum: u8, unix_seconds: i64, fraction: u32) -> Vec<u8> {
        let mut packet = vec![0u8; PACKET_BYTES];
        packet[0] = header;
        packet[1] = stratum;
        let ntp_seconds = (unix_seconds + NTP_TO_UNIX_SECONDS) as u32;
        packet[40..44].copy_from_slice(&ntp_seconds.to_be_bytes());
        packet[44..48].copy_from_slice(&fraction.to_be_bytes());
        packet
    }

    /// 2026-09-26 07:30:15 UTC.
    const MORNING: i64 = 1_790_407_815;

    #[test]
    fn reads_the_time_the_server_sent_rounded_to_the_second() {
        assert_eq!(decode(&answer(SERVER_HEADER, 2, MORNING, 0)), Ok(UtcTime::from_unix_seconds(MORNING)));
        assert_eq!(decode(&answer(SERVER_HEADER, 2, MORNING, 3 << 30)), Ok(UtcTime::from_unix_seconds(MORNING + 1)));
    }

    #[test]
    fn counts_on_past_2036() {
        let in_2040 = NTP_TO_UNIX_SECONDS;
        assert_eq!(decode(&answer(SERVER_HEADER, 2, in_2040, 0)), Ok(UtcTime::from_unix_seconds(in_2040)));
    }

    #[test]
    fn a_server_answer_is_read_back_as_the_time_it_was_made_at() {
        for unix_seconds in [MORNING, NTP_TO_UNIX_SECONDS] {
            let time = DateTime::from_unix_seconds(unix_seconds);
            assert_eq!(decode(&server_answer(time)), Ok(UtcTime::from_unix_seconds(unix_seconds)), "{time:?}");
        }
    }

    #[test]
    fn refuses_what_is_not_a_synchronised_server() {
        assert!(decode(&answer(0b00_100_011, 2, MORNING, 0)).is_err(), "a client's packet");
        assert!(decode(&answer(0b11_100_100, 2, MORNING, 0)).is_err(), "unsynchronised");
        assert!(decode(&answer(SERVER_HEADER, 0, MORNING, 0)).is_err(), "kiss-o'-death");
        assert!(decode(&answer(SERVER_HEADER, 2, MORNING, 0)[..47]).is_err(), "cut short");
    }

    struct StubDatagrams {
        answer: Vec<u8>,
        request: Option<(String, u16, Vec<u8>)>,
    }

    impl Datagrams for StubDatagrams {
        fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8]) -> Result<usize, Unavailable> {
            self.request = Some((host.into(), port, request.to_vec()));
            answer[..self.answer.len()].copy_from_slice(&self.answer);
            Ok(self.answer.len())
        }
    }

    #[test]
    fn asks_as_a_version_4_client() {
        let mut server = NtpServer::new(StubDatagrams { answer: answer(SERVER_HEADER, 1, MORNING, 0), request: None });
        assert_eq!(server.fetch(), Ok(UtcTime::from_unix_seconds(MORNING)));
        let (host, port, request) = server.network.request.unwrap();
        assert_eq!((host.as_str(), port), (SERVER, PORT));
        assert_eq!(request.len(), PACKET_BYTES);
        assert_eq!(request[0], CLIENT_REQUEST);
    }
}
