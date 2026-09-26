use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::i2c::{I2c, I2cConfig, I2cDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::TickType_t;
use hal::bus::I2cBus;
use hal::Fault;

use crate::or_fault::OrFault;

const TIMEOUT: TickType_t = TickType::new_millis(50).ticks();

/// A handle on the I2C bus; every device driver on the bus holds a clone.
#[derive(Clone)]
pub struct I2cMaster(Arc<Mutex<I2cDriver<'static>>>);

impl I2cMaster {
    pub fn new(
        i2c: impl I2c + 'static,
        sda: impl InputPin + OutputPin + 'static,
        scl: impl InputPin + OutputPin + 'static,
    ) -> Result<Self, Fault> {
        let config = I2cConfig::new().baudrate(100.kHz().into());
        let driver = I2cDriver::new(i2c, sda, scl, &config).or_fault("opening I2C")?;
        Ok(Self(Arc::new(Mutex::new(driver))))
    }

    /// One transaction: nothing else on the bus can come between the write and the read.
    pub(crate) fn write_read(&self, address: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Fault> {
        let mut bus = self.0.lock().map_err(|_| Fault::new("I2C bus poisoned"))?;
        bus.write_read(address, bytes, buffer, TIMEOUT).or_fault("I2C read")
    }
}

impl I2cBus for I2cMaster {
    fn scan(&mut self) -> Vec<u8> {
        let Ok(mut bus) = self.0.lock() else {
            return Vec::new();
        };
        (0x08..=0x77).filter(|&address| bus.write(address, &[], TIMEOUT).is_ok()).collect()
    }
}
