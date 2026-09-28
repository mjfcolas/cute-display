use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution};
use esp_idf_svc::hal::units::FromValueType;
use hal::light::{Brightness, DimmableLight};
use hal::Fault;

use crate::or_fault::OrFault;

/// Smooth from off to full on the board's lights, with no audible buzz.
const FREQUENCY_HZ: u32 = 1_000;
const RESOLUTION: Resolution = Resolution::Bits10;

pub type SharedTimer<T> = &'static LedcTimerDriver<'static, <T as LedcTimer>::SpeedMode>;

pub fn shared_timer<T: LedcTimer + 'static>(timer: T) -> Result<SharedTimer<T>, Fault> {
    let config = TimerConfig::new().frequency(FREQUENCY_HZ.Hz()).resolution(RESOLUTION);
    let driver = LedcTimerDriver::new(timer, &config).or_fault("LEDC timer")?;
    Ok(Box::leak(Box::new(driver)))
}

pub struct LedcLight {
    channel: LedcDriver<'static>,
    brightness: Brightness,
}

impl LedcLight {
    pub fn new<C: LedcChannel + 'static>(
        channel: C,
        timer: &'static LedcTimerDriver<'static, C::SpeedMode>,
        pin: impl OutputPin + 'static,
    ) -> Result<Self, Fault> {
        let mut light = Self { channel: LedcDriver::new(channel, timer, pin).or_fault("LEDC channel")?, brightness: Brightness::FULL };
        light.set_brightness(Brightness::OFF)?;
        Ok(light)
    }
}

impl DimmableLight for LedcLight {
    fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
        let duty = self.channel.get_max_duty() * u32::from(brightness.as_percent()) / 100;
        self.channel.set_duty(duty).or_fault("LEDC duty")?;
        self.brightness = brightness;
        Ok(())
    }

    fn brightness(&self) -> Brightness {
        self.brightness
    }
}
