use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::AdcChannel;
use esp_idf_svc::hal::gpio::{Input, PinDriver};
use hal::power::PowerMonitor;
use hal::Fault;

use crate::or_fault::OrFault;

const SAMPLES_PER_READING: u32 = 8;

pub type SharedAdc<C> = &'static AdcDriver<'static, <C as AdcChannel>::AdcUnit>;

/// External power detected as a logic level; the battery sensed through the ADC.
pub struct AdcPowerMonitor<C: AdcChannel> {
    external_power: PinDriver<'static, Input>,
    battery: AdcChannelDriver<'static, C, SharedAdc<C>>,
}

impl<C: AdcChannel> AdcPowerMonitor<C> {
    pub fn new(external_power: PinDriver<'static, Input>, battery: AdcChannelDriver<'static, C, SharedAdc<C>>) -> Self {
        Self { external_power, battery }
    }
}

impl<C: AdcChannel> PowerMonitor for AdcPowerMonitor<C> {
    fn on_external_power(&self) -> bool {
        self.external_power.is_high()
    }

    fn battery_sense_millivolts(&mut self) -> Result<u32, Fault> {
        let mut total = 0;
        for _ in 0..SAMPLES_PER_READING {
            total += u32::from(self.battery.read().or_fault("battery ADC")?);
        }
        Ok(total / SAMPLES_PER_READING)
    }
}
