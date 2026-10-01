use std::thread;
use std::time::Duration;

use hal::light::{Brightness, DimmableLight};

use crate::bench::{eventually, Image, Setup, ALARM_CONF, GENERAL_CONF};

const MINUTE: Duration = Duration::from_secs(60);

/// Weather in front, an alarm at seven on Saturday morning, the RTC at six.
fn waking_at_seven() -> Image {
    let setup = Setup::default()
        .with_file(GENERAL_CONF, "time_zone = UTC0\napps = weather, alarm\n")
        .with_file(ALARM_CONF, "enabled = yes\nsaturday = 07:00\n");
    Image::start(setup)
}

#[test]
fn half_an_hour_before_the_alarm_the_lights_come_up_like_a_sunrise() {
    let image = waking_at_seven();
    image.advance(25 * MINUTE);
    assert_eq!(image.reading_lamp.brightness(), Brightness::OFF, "still night at 06:25");
    image.advance(20 * MINUTE);
    let at_quarter_to = image.reading_lamp.brightness();
    assert!(at_quarter_to > Brightness::OFF, "dawn at 06:45");
    image.advance(10 * MINUTE);
    assert!(image.reading_lamp.brightness() > at_quarter_to, "brighter at 06:55");
}

#[test]
fn at_its_time_the_alarm_comes_to_the_front_and_rings_and_the_long_button_snoozes_it_nine_minutes() {
    let image = waking_at_seven();
    assert_eq!(image.screen_line("front").as_deref(), Some("front weather"));
    image.advance(60 * MINUTE);
    assert_eq!(image.screen_line("front").as_deref(), Some("front alarm"));
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Good morning!"));
    assert!(eventually(|| image.speaker.sounds_heard() > 0), "it rings");

    image.press_long_button();
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Snoozing until 07:09"));
    thread::sleep(Duration::from_millis(50));
    let rung = image.speaker.sounds_started();
    image.advance(8 * MINUTE);
    thread::sleep(Duration::from_millis(50));
    assert_eq!(image.speaker.sounds_started(), rung, "silent while it snoozes");
    image.advance(2 * MINUTE);
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Good morning!"));
    assert!(eventually(|| image.speaker.sounds_started() > rung), "it rings again at 07:09");
}
