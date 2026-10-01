use std::time::Duration;

use hal::clock::DateTime;
use hal_testing::udp::StubUdpClient;

use crate::bench::{Image, RtcAtStart, Setup, WIFI_CONF};

const MORNING: DateTime = DateTime { year: 2026, month: 9, day: 26, hour: 7, minute: 30, second: 15 };

fn with_wifi(setup: Setup) -> Setup {
    setup.with_file(WIFI_CONF, "ssid = Home\npassword = s3cret\n")
}

#[test]
fn a_stopped_rtc_is_set_from_the_network_time() {
    let image = Image::start(with_wifi(Setup { rtc: RtcAtStart::Stopped, udp: StubUdpClient::ntp_at(MORNING), ..Setup::default() }));
    image.advance(Duration::from_secs(1));
    assert_eq!(image.rtc.time().map(DateTime::unix_seconds), Some(MORNING.unix_seconds() + 1), "set, then a second on");
    assert_eq!(image.screen_line("time").as_deref(), Some("time 07:30"));
}

#[test]
fn without_an_answer_the_network_time_is_asked_again_ten_minutes_later() {
    let image = Image::start(with_wifi(Setup::default()));
    image.advance(Duration::from_secs(1));
    assert_eq!(image.udp.asked().len(), 1);
    image.advance(Duration::from_secs(9 * 60));
    assert_eq!(image.udp.asked().len(), 1, "not before ten minutes");
    image.advance(Duration::from_secs(60));
    assert_eq!(image.udp.asked().len(), 2);
}
