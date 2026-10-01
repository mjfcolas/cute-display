use std::time::Duration;

use crate::bench::{Image, Setup, GENERAL_CONF};

#[test]
fn the_first_app_general_conf_names_comes_to_the_front() {
    let image = Image::start(Setup::default().with_file(GENERAL_CONF, "time_zone = UTC0\napps = weather, alarm\n"));
    assert_eq!(image.screen_line("front").as_deref(), Some("front weather"));
}

#[test]
fn without_a_card_the_image_runs_its_apps_in_their_order_in_central_european_time() {
    let image = Image::start(Setup { card: None, ..Setup::default() });
    assert_eq!(image.screen_line("front").as_deref(), Some("front alarm"));
    image.advance(Duration::from_secs(60));
    assert_eq!(image.screen_line("time").as_deref(), Some("time 08:01"));
}
