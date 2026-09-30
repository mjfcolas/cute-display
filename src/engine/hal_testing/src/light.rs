use std::sync::{Arc, Mutex};

use hal::light::{Brightness, DimmableLight};
use hal::Fault;

use crate::shared::lock;

#[derive(Clone, Default)]
pub struct FakeLight(Arc<Mutex<Brightness>>);

impl FakeLight {
    pub fn at(brightness: Brightness) -> Self {
        Self(Arc::new(Mutex::new(brightness)))
    }
}

impl DimmableLight for FakeLight {
    fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
        *lock(&self.0) = brightness;
        Ok(())
    }

    fn brightness(&self) -> Brightness {
        *lock(&self.0)
    }
}

/// Checks a light against `DimmableLight`'s contract.
pub fn check_contract(light: &mut impl DimmableLight) {
    for brightness in [Brightness::FULL, Brightness::percent(30), Brightness::OFF] {
        assert_eq!(light.set_brightness(brightness), Ok(()));
        assert_eq!(light.brightness(), brightness, "a light is at the brightness it was set to");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_light_keeps_the_contract() {
        check_contract(&mut FakeLight::default());
    }
}
