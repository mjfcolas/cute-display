use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use hal::light::{Brightness, DimmableLight};
use hal::Fault;

#[derive(Clone, Default)]
pub struct SimulatedLight {
    percent: Arc<AtomicU8>,
}

impl DimmableLight for SimulatedLight {
    fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
        self.percent.store(brightness.as_percent(), Ordering::Relaxed);
        Ok(())
    }

    fn brightness(&self) -> Brightness {
        Brightness::percent(self.percent.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use hal_testing::light;

    use super::*;

    #[test]
    fn a_simulated_light_keeps_the_contract() {
        light::check_contract(&mut SimulatedLight::default());
    }
}
