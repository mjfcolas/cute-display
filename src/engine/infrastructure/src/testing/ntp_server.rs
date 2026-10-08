use hal::clock::DateTime;

use crate::ntp::{NTP_TO_UNIX_SECONDS, PACKET_BYTES, SERVER_HEADER, SERVER_STRATUM, TRANSMIT_TIMESTAMP};

/// What a server answers when it is `time`, on the second.
pub fn answer(time: DateTime) -> [u8; PACKET_BYTES] {
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
