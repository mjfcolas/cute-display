use std::time::Duration;

use hal::clock::DateTime;
use hal_testing::udp::StubUdpClient;

use crate::bench::{Image, RtcAtStart, Setup, GENERAL_CONF, MINUTE};

const MORNING: DateTime = DateTime { year: 2026, month: 9, day: 26, hour: 7, minute: 30, second: 15 };

#[test]
fn a_stopped_rtc_is_set_from_the_network_time() {
    let image = Image::start(Setup { rtc: RtcAtStart::Stopped, udp: StubUdpClient::ntp_at(MORNING), ..Setup::default() }.with_wifi());
    image.advance(Duration::from_secs(1));
    assert_eq!(image.rtc.time().map(DateTime::unix_seconds), Some(MORNING.unix_seconds() + 1), "set, then a second on");
    assert_eq!(image.screen_line("time").as_deref(), Some("time 07:30"));
}

#[test]
fn without_an_answer_the_network_time_is_asked_again_ten_minutes_later() {
    let image = Image::start(Setup::default().with_wifi());
    image.advance(Duration::from_secs(1));
    assert_eq!(image.udp.asked().len(), 1);
    image.advance(9 * MINUTE);
    assert_eq!(image.udp.asked().len(), 1, "not before ten minutes");
    image.advance(MINUTE);
    assert_eq!(image.udp.asked().len(), 2);
}

#[test]
fn a_time_zone_changed_on_the_card_shows_within_a_minute_without_a_restart() {
    let image = Image::start(Setup::default());
    assert_eq!(image.screen_line("time").as_deref(), Some("time 06:00"));
    image.put_file(GENERAL_CONF, "time_zone = CET-1CEST,M3.5.0,M10.5.0/3\n");
    image.advance(MINUTE);
    assert_eq!(image.screen_line("time").as_deref(), Some("time 08:01"), "summer time in Paris");
}

#[test]
fn an_unreadable_rtc_leaves_the_time_unknown_and_the_image_running() {
    let image = Image::start(Setup { rtc: RtcAtStart::Unreadable, ..Setup::default() });
    image.advance(MINUTE);
    assert_eq!(image.screen_line("time").as_deref(), Some("time --:--"));
    assert_eq!(image.screen_line("date").as_deref(), Some("date The time is not known yet"));
}
