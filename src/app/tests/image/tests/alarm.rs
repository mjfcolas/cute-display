use hal::light::{Brightness, DimmableLight};

use crate::bench::{eventually, let_the_speaker_settle, Image, Setup, ALARM_CONF, GENERAL_CONF, MINUTE};

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
    let_the_speaker_settle();
    let rung = image.speaker.sounds_started();
    image.advance(8 * MINUTE);
    let_the_speaker_settle();
    assert_eq!(image.speaker.sounds_started(), rung, "silent while it snoozes");
    image.advance(2 * MINUTE);
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Good morning!"));
    assert!(eventually(|| image.speaker.sounds_started() > rung), "it rings again at 07:09");
}

#[test]
fn left_alone_the_alarm_rings_fifteen_minutes_then_stops() {
    let image = waking_at_seven();
    image.advance(60 * MINUTE + 14 * MINUTE);
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Good morning!"), "still ringing at 07:14");
    image.advance(MINUTE);
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Alarm Saturday at 07:00"), "next week's at 07:15");
    let_the_speaker_settle();
    let stopped = image.speaker.sounds_started();
    image.advance(10 * MINUTE);
    let_the_speaker_settle();
    assert_eq!(image.speaker.sounds_started(), stopped, "silent once stopped");
}

#[test]
fn holding_the_yellow_and_the_long_buttons_together_stops_the_alarm_and_its_light() {
    let image = waking_at_seven();
    image.advance(60 * MINUTE);
    assert!(image.reading_lamp.brightness() > Brightness::OFF);
    image.hold_yellow_and_long_buttons();
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Alarm Saturday at 07:00"));
    assert_eq!(image.reading_lamp.brightness(), Brightness::OFF);
}

#[test]
fn a_wake_up_time_set_with_the_controls_is_kept_on_the_card_and_rings() {
    let image = Image::start(Setup::default());
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Alarm off"));
    let saturday = |image: &Image| image.screen_lines("row").into_iter().find(|row| row.starts_with("row Saturday"));

    image.press_long_button();
    assert_eq!(saturday(&image).as_deref(), Some("row Saturday off *"), "the settings open on today");
    image.press_long_button();
    assert_eq!(saturday(&image).as_deref(), Some("row Saturday [07]:00 *"), "its hour, first at seven");
    image.press_long_button();
    assert_eq!(saturday(&image).as_deref(), Some("row Saturday 07:[00] *"), "then its minutes");
    image.press_long_button();
    assert_eq!(saturday(&image).as_deref(), Some("row Saturday 07:00 *"), "set");
    image.press_yellow_button();
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Alarm off"), "back on the clock");
    image.press_yellow_button();
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Alarm today at 07:00"), "switched on");

    let conf = image.file(ALARM_CONF).unwrap_or_default();
    assert!(conf.contains("enabled = yes") && conf.contains("saturday = 07:00"), "{conf}");
    image.advance(60 * MINUTE);
    assert_eq!(image.screen_line("alarm").as_deref(), Some("alarm Good morning!"));
    assert!(eventually(|| image.speaker.sounds_heard() > 0), "it rings");
}
