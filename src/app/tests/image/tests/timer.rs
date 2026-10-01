use hal::light::{Brightness, DimmableLight};

use crate::bench::{eventually, Image, Setup, GENERAL_CONF, MINUTE, TIMER_CONF};

fn timer_with(conf: &str) -> Image {
    Image::start(Setup::default().with_file(GENERAL_CONF, "time_zone = UTC0\napps = timer, weather\n").with_file(TIMER_CONF, conf))
}

#[test]
fn a_countdown_started_with_yellow_comes_back_to_the_front_and_rings_until_stopped() {
    let image = timer_with("countdown = 2\n");
    assert_eq!(image.screen_line("count").as_deref(), Some("count 2 min"));
    image.press_yellow_button();
    assert_eq!(image.screen_line("status").as_deref(), Some("status Counting down"));
    image.click_wheel();
    assert_eq!(image.screen_line("front").as_deref(), Some("front system"));
    image.advance(MINUTE);
    assert_eq!(image.screen_line("front").as_deref(), Some("front system"), "a minute left");
    image.advance(MINUTE);
    assert_eq!(image.screen_line("front").as_deref(), Some("front timer"));
    assert_eq!(image.screen_line("status").as_deref(), Some("status Time is up"));
    assert!(eventually(|| image.speaker.sounds_heard() > 0), "it rings");
    image.press_yellow_button();
    assert_eq!(image.screen_line("count").as_deref(), Some("count 2 min"));
}

#[test]
fn a_nap_keeps_the_lights_off_raises_them_over_its_last_minutes_and_wakes() {
    let image = timer_with("mode = nap\nnap = 10\n");
    image.press_yellow_button();
    image.advance(2 * MINUTE + MINUTE / 2);
    assert_eq!(image.reading_lamp.brightness(), Brightness::OFF, "dark at first");
    image.advance(6 * MINUTE);
    assert!(image.reading_lamp.brightness() > Brightness::OFF, "rising over the last three minutes");
    image.advance(MINUTE + MINUTE / 2);
    assert_eq!(image.screen_line("status").as_deref(), Some("status Time to wake up"));
    assert!(eventually(|| image.speaker.sounds_heard() > 0), "it rings");
    image.press_yellow_button();
    assert_eq!(image.screen_line("status").as_deref(), Some("status Lights out, then the light rises"));
}

#[test]
fn the_mode_and_the_minutes_set_are_kept_on_the_card() {
    let image = timer_with("");
    image.press_long_button();
    image.turn_wheel(1);
    assert_eq!(image.screen_line("count").as_deref(), Some("count 30 min"));
    assert_eq!(image.file(TIMER_CONF).as_deref(), Some("mode = pomodoro\ncountdown = 10\npomodoro = 30\nnap = 20\n"));
}
