use std::time::Duration;

use hal::light::{Brightness, DimmableLight};

use crate::bench::{eventually, Image, Setup, BREATHE_CONF, GENERAL_CONF, MINUTE};

const SECOND: Duration = Duration::from_secs(1);

fn breathing_app() -> Image {
    Image::start(Setup::default().with_file(GENERAL_CONF, "time_zone = UTC0\napps = breathe\n"))
}

#[test]
fn started_with_yellow_the_lights_rise_and_fall_with_each_breath_and_a_breath_of_sound_plays_until_stopped() {
    let image = breathing_app();
    image.press_yellow_button();
    assert!(eventually(|| image.speaker.sounds_started() > 0), "it plays");
    image.advance(5 * SECOND);
    let breathed_in = image.reading_lamp.brightness();
    assert!(breathed_in > Brightness::OFF, "up once breathed in");
    image.advance(5 * SECOND);
    assert!(image.reading_lamp.brightness() < breathed_in, "down once breathed out");
    image.advance(5 * SECOND);
    assert!(image.reading_lamp.brightness() > Brightness::OFF, "and up again");
    image.advance(MINUTE);
    assert_eq!(image.screen_line("status").as_deref(), Some("status Breathing for 1 min"));
    image.press_yellow_button();
    assert_eq!(image.screen_line("status").as_deref(), Some("status Until you stop it"));
    assert_eq!(image.reading_lamp.brightness(), Brightness::OFF);
}

#[test]
fn the_rhythm_and_the_volume_are_kept_on_the_card() {
    let image = breathing_app();
    image.turn_wheel(1);
    image.press_yellow_button();
    image.turn_wheel(-2);
    assert_eq!(image.file(BREATHE_CONF).as_deref(), Some("rhythm = 4-7-8\nvolume = 20\n"));
}
