use hal::light::{Brightness, DimmableLight};

use crate::bench::{eventually, let_the_speaker_settle, Image, Setup, GENERAL_CONF, MINUTE, ZEN_CONF};

/// Weather in front, a session of an hour every day at half past six, the RTC at six.
fn relaxing_at_half_past_six() -> Image {
    let setup = Setup::default()
        .with_file(GENERAL_CONF, "time_zone = UTC0\napps = weather, zen\n")
        .with_file(ZEN_CONF, "start = 06:30\nhours = 1\nsound = brown noise\nvolume = 30\n");
    Image::start(setup)
}

#[test]
fn at_its_time_a_session_comes_to_the_front_plays_and_dims_the_lights_then_ends_after_its_hours() {
    let image = relaxing_at_half_past_six();
    assert_eq!(image.screen_line("front").as_deref(), Some("front weather"));
    image.advance(30 * MINUTE);
    assert_eq!(image.screen_line("front").as_deref(), Some("front zen"));
    assert_eq!(image.screen_line("session").as_deref(), Some("session 1 h left"));
    assert!(eventually(|| image.speaker.sounds_started() > 0), "it plays");
    let at_start = image.reading_lamp.brightness();
    assert!(at_start > Brightness::OFF, "the lights on at 06:30");
    image.advance(15 * MINUTE);
    let dimmed = image.reading_lamp.brightness();
    assert!(Brightness::OFF < dimmed && dimmed < at_start, "dimmer at 06:45");
    image.advance(15 * MINUTE);
    assert_eq!(image.reading_lamp.brightness(), Brightness::OFF, "out at 07:00");

    image.advance(30 * MINUTE);
    assert_eq!(image.screen_line("session").as_deref(), Some("session Every day at 06:30 for 1 h"), "over at 07:30");
    let_the_speaker_settle();
    let played = image.speaker.sounds_started();
    image.advance(10 * MINUTE);
    let_the_speaker_settle();
    assert_eq!(image.speaker.sounds_started(), played, "silent once over");
}

#[test]
fn the_yellow_button_starts_a_session_the_wheel_turns_it_up_and_the_card_keeps_the_volume() {
    let image = Image::start(Setup::default().with_file(GENERAL_CONF, "time_zone = UTC0\napps = zen\n"));
    assert_eq!(image.screen_line("session").as_deref(), Some("session No time set; 1 h when started"));
    image.press_yellow_button();
    assert_eq!(image.screen_line("session").as_deref(), Some("session 1 h left"));
    assert_eq!(image.screen_line("volume").as_deref(), Some("volume 30%"));
    image.turn_wheel(2);
    assert_eq!(image.screen_line("volume").as_deref(), Some("volume 40%"));
    assert!(image.file(ZEN_CONF).unwrap_or_default().contains("volume = 40"));
    image.press_yellow_button();
    assert_eq!(image.screen_line("session").as_deref(), Some("session No time set; 1 h when started"));
}
