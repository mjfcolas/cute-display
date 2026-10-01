use std::time::Duration;

use hal_testing::udp::StubUdpClient;
use infrastructure::ntp;

use crate::bench::{Image, Setup, OPEN_METEO, SATURDAY_AT_SIX};

#[test]
fn the_wifi_is_joined_once_for_the_fetches_at_start_and_left_once_idle_a_minute() {
    let image = Image::start(Setup { udp: StubUdpClient::answering(ntp::server_answer(SATURDAY_AT_SIX)), ..Setup::default() }.with_recorded_forecast().with_wifi());
    image.advance(Duration::from_secs(1));
    assert_eq!(image.udp.asked().len(), 1, "the network time");
    assert_eq!(image.http.asked().iter().filter(|url| url.starts_with(OPEN_METEO)).count(), 2, "the alarm's forecast and the weather's");
    assert_eq!(image.wifi.joins(), 1, "all on one join");
    image.advance(Duration::from_secs(58));
    assert!(image.wifi.is_joined(), "idle 59 s");
    image.advance(Duration::from_secs(2));
    assert!(!image.wifi.is_joined(), "left once idle a minute");
}
