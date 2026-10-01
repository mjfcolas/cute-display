use std::time::Duration;

use hal_testing::http::StubHttpClient;

use crate::bench::{Image, Setup, GENERAL_CONF, MINUTE};

const ADSB_FI: &str = "https://opendata.adsb.fi/";
const NO_AIRCRAFT: &str = r#"{"ac": []}"#;

fn radar_in_front(http: StubHttpClient) -> Image {
    let setup = Setup { http, ..Setup::default() }
        .with_file(GENERAL_CONF, "place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\ntime_zone = UTC0\napps = radar, weather\n")
        .with_wifi();
    Image::start(setup)
}

fn asked_adsb_fi(image: &Image) -> usize {
    image.http.asked().iter().filter(|url| url.starts_with(ADSB_FI)).count()
}

#[test]
fn the_sky_is_asked_every_fifteen_seconds_but_only_while_the_radar_is_in_front() {
    let image = radar_in_front(StubHttpClient::default().answering(ADSB_FI, NO_AIRCRAFT));
    image.advance(MINUTE);
    assert_eq!(asked_adsb_fi(&image), 5, "at 0, 15, 30, 45 and 60 s");
    image.click_wheel();
    assert_eq!(image.screen_line("front").as_deref(), Some("front system"));
    let in_front = asked_adsb_fi(&image);
    image.advance(MINUTE);
    assert_eq!(asked_adsb_fi(&image), in_front, "not asked behind the system app");
}

#[test]
fn a_sky_that_does_not_answer_is_said() {
    let image = radar_in_front(StubHttpClient::default());
    image.advance(Duration::from_secs(1));
    assert!(image.screen_line("trouble").is_some_and(|trouble| trouble.starts_with("trouble offline:")), "{:?}", image.said_on_screen());
}
