//! An NTP server that answers the true time of the simulated world.

use core::time::Duration;

use hal::clock::DateTime;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::ntp::{self, PACKET_BYTES};

use crate::clock::TrueTime;

pub struct SimulatedNtpServer(pub TrueTime);

impl UdpClient for SimulatedNtpServer {
    fn exchange(&mut self, _host: &str, _port: u16, _request: &[u8], answer: &mut [u8], _timeout: Duration) -> Result<usize, Fault> {
        let room = answer.get_mut(..PACKET_BYTES).ok_or_else(|| Fault::new("no room for an NTP answer"))?;
        room.copy_from_slice(&ntp::server_answer(DateTime::from_unix_seconds(self.0.unix_seconds())));
        Ok(PACKET_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use hal::clock::RealTimeClock;

    use super::*;
    use crate::clock::HostClock;
    use crate::steady::ScaledClock;

    const MORNING: i64 = 1_790_407_815;

    fn asked(server: &mut SimulatedNtpServer) -> [u8; PACKET_BYTES] {
        let mut answer = [0xff; 2 * PACKET_BYTES];
        let length = server.exchange("pool.ntp.org", 123, &[0; PACKET_BYTES], &mut answer, Duration::from_secs(1)).unwrap();
        answer[..length].try_into().unwrap()
    }

    #[test]
    fn answers_the_true_time_even_once_the_rtc_is_set_otherwise() {
        let time = TrueTime::starting_at(ScaledClock::new(NonZeroU32::MIN), DateTime::from_unix_seconds(MORNING));
        HostClock::new(time).set(DateTime::from_unix_seconds(0)).unwrap();
        let answer = asked(&mut SimulatedNtpServer(time));
        let in_time = [MORNING, MORNING + 1].map(|seconds| ntp::server_answer(DateTime::from_unix_seconds(seconds)));
        assert!(in_time.contains(&answer));
    }
}
