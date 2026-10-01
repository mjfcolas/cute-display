use std::time::Duration;

use hal::light::{Brightness, DimmableLight};

use crate::bench::{Image, Setup, MINUTE};

#[test]
fn a_touch_lights_the_screen_for_ten_seconds() {
    let image = Image::start(Setup::default());
    image.advance(MINUTE);
    assert_eq!(image.front_light.brightness(), Brightness::OFF, "dark until touched");
    image.turn_wheel(1);
    assert!(image.front_light.brightness() > Brightness::OFF, "lit by a touch");
    image.advance(Duration::from_secs(9));
    assert!(image.front_light.brightness() > Brightness::OFF, "still lit nine seconds on");
    image.advance(Duration::from_secs(2));
    assert_eq!(image.front_light.brightness(), Brightness::OFF, "dark again after ten");
}
