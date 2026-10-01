use std::time::Duration;

use crate::bench::{Image, Setup, GENERAL_CONF, MINUTE, OPEN_METEO, RECORDED_FORECAST};

fn weather_in_front(setup: Setup) -> Image {
    Image::start(setup.with_file(GENERAL_CONF, "latitude = 48.85\nlongitude = 2.35\ntime_zone = UTC0\napps = weather\n").with_wifi())
}

fn asked_open_meteo(image: &Image) -> usize {
    image.http.asked().iter().filter(|url| url.starts_with(OPEN_METEO)).count()
}

fn status(image: &Image) -> String {
    image.screen_line("status").unwrap_or_default()
}

#[test]
fn the_forecast_fetched_at_start_is_shown() {
    let image = weather_in_front(Setup::default().with_recorded_forecast());
    image.advance(Duration::from_secs(1));
    assert_eq!(image.screen_line("now").as_deref(), Some("now Cloudy 19°"));
    assert_eq!(image.screen_line("day").as_deref(), Some("day 12° / 17° rain 85%"), "Saturday's");
    assert!(status(&image).starts_with("status updated just now"), "{}", status(&image));
}

#[test]
fn the_forecast_is_fetched_again_an_hour_later() {
    let image = weather_in_front(Setup::default().with_recorded_forecast());
    image.advance(Duration::from_secs(1));
    let at_start = asked_open_meteo(&image);
    image.advance(58 * MINUTE);
    assert_eq!(asked_open_meteo(&image), at_start, "not before an hour");
    image.advance(2 * MINUTE);
    assert!(asked_open_meteo(&image) > at_start);
}

#[test]
fn a_failed_fetch_is_said_and_mended_ten_minutes_later() {
    let image = weather_in_front(Setup::default());
    image.advance(Duration::from_secs(1));
    assert!(status(&image).starts_with("status offline:"), "{}", status(&image));
    image.http.starts_answering(OPEN_METEO, RECORDED_FORECAST);
    image.advance(9 * MINUTE);
    assert!(status(&image).starts_with("status offline:"), "not tried again before ten minutes: {}", status(&image));
    image.advance(MINUTE);
    assert!(status(&image).starts_with("status updated just now"), "{}", status(&image));
}
