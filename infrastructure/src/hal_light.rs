use domain::lighting::{Level, Light};
use hal::light::{Brightness, DimmableLight};

/// A light of the domain on a dimmable light of the board.
pub struct HalLight<L> {
    light: L,
}

impl<L: DimmableLight> HalLight<L> {
    pub fn new(light: L) -> Self {
        Self { light }
    }
}

impl<L: DimmableLight + Send> Light for HalLight<L> {
    fn shine(&mut self, level: Level) {
        if let Err(fault) = self.light.set_brightness(Brightness::percent(level.as_percent())) {
            log::warn!("light: {fault}");
        }
    }
}

#[cfg(test)]
mod tests {
    use hal::Fault;

    use super::*;

    #[derive(Default)]
    struct FakeLight(Brightness);

    impl DimmableLight for FakeLight {
        fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
            self.0 = brightness;
            Ok(())
        }
        fn brightness(&self) -> Brightness {
            self.0
        }
    }

    #[test]
    fn a_level_is_the_same_percentage_of_brightness() {
        let mut light = HalLight::new(FakeLight::default());
        light.shine(Level::percent(30));
        assert_eq!(light.light.brightness().as_percent(), 30);
        light.shine(Level::OFF);
        assert_eq!(light.light.brightness(), Brightness::OFF);
    }
}
