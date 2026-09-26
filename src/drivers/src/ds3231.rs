//! Maxim DS3231: a real-time clock with a temperature sensor of its own, read-only.

use hal::clock::{ClockReading, DateTime};
use hal::thermometer::Temperature;

pub const ADDRESS: u8 = 0x68;
pub const SECONDS_REGISTER: u8 = 0x00;
/// Time, alarms, control and status: 0x00 to 0x0F.
pub const CLOCK_REGISTERS: usize = 0x10;
pub const TEMPERATURE_REGISTER: u8 = 0x11;
/// Clock registers, then aging offset and temperature: everything the chip holds.
pub const ALL_REGISTERS: usize = 0x13;

const STATUS: usize = 0x0f;
const OSCILLATOR_STOPPED: u8 = 0x80;
const CENTURY: u8 = 0x80;
const TWELVE_HOUR_MODE: u8 = 0x40;
const PM: u8 = 0x20;

fn bcd(b: u8) -> u8 {
    (b >> 4) * 10 + (b & 0x0f)
}

fn hour(register: u8) -> u8 {
    if register & TWELVE_HOUR_MODE == 0 {
        return bcd(register & 0x3f);
    }
    let h = bcd(register & 0x1f) % 12;
    if register & PM != 0 { h + 12 } else { h }
}

pub fn decode_clock(registers: &[u8; CLOCK_REGISTERS], alarm_raised: bool) -> ClockReading {
    let r = |i: usize| registers.get(i).copied().unwrap_or(0);
    ClockReading {
        time: DateTime {
            year: 2000 + u16::from(bcd(r(6))) + if r(5) & CENTURY != 0 { 100 } else { 0 },
            month: bcd(r(5) & 0x1f),
            day: bcd(r(4) & 0x3f),
            hour: hour(r(2)),
            minute: bcd(r(1) & 0x7f),
            second: bcd(r(0) & 0x7f),
        },
        oscillator_stopped: r(STATUS) & OSCILLATOR_STOPPED != 0,
        alarm_raised,
    }
}

/// Whole degrees as a signed byte, then quarters in the top two bits of the next.
pub fn decode_temperature([degrees, quarters]: [u8; 2]) -> Temperature {
    Temperature { quarter_degrees_celsius: i16::from(degrees as i8) * 4 + i16::from(quarters >> 6) }
}

#[cfg(target_os = "espidf")]
pub use chip::{Ds3231Clock, Ds3231Thermometer};

#[cfg(target_os = "espidf")]
mod chip {
    use esp_idf_svc::hal::gpio::{Input, PinDriver};
    use hal::clock::{ClockReading, RealTimeClock};
    use hal::thermometer::{Temperature, Thermometer};
    use hal::Fault;

    use super::{
        decode_clock, decode_temperature, ADDRESS, ALL_REGISTERS, CLOCK_REGISTERS, SECONDS_REGISTER, TEMPERATURE_REGISTER,
    };
    use crate::i2c::I2cMaster;

    pub struct Ds3231Clock {
        bus: I2cMaster,
        /// Open drain, pulled up on the board: low while an alarm flag is set.
        interrupt: PinDriver<'static, Input>,
    }

    impl Ds3231Clock {
        pub fn new(bus: I2cMaster, interrupt: PinDriver<'static, Input>) -> Self {
            Self { bus, interrupt }
        }

        /// Every register as it is, alarms included, for a backup.
        pub fn registers(&mut self) -> Result<[u8; ALL_REGISTERS], Fault> {
            let mut registers = [0u8; ALL_REGISTERS];
            self.bus.write_read(ADDRESS, &[SECONDS_REGISTER], &mut registers)?;
            Ok(registers)
        }
    }

    impl RealTimeClock for Ds3231Clock {
        fn read(&mut self) -> Result<ClockReading, Fault> {
            // One transaction, so the time cannot tear across a rollover.
            let mut registers = [0u8; CLOCK_REGISTERS];
            self.bus.write_read(ADDRESS, &[SECONDS_REGISTER], &mut registers)?;
            Ok(decode_clock(&registers, self.interrupt.is_low()))
        }
    }

    pub struct Ds3231Thermometer {
        bus: I2cMaster,
    }

    impl Ds3231Thermometer {
        pub fn new(bus: I2cMaster) -> Self {
            Self { bus }
        }
    }

    impl Thermometer for Ds3231Thermometer {
        fn temperature(&mut self) -> Result<Temperature, Fault> {
            let mut registers = [0u8; 2];
            self.bus.write_read(ADDRESS, &[TEMPERATURE_REGISTER], &mut registers)?;
            Ok(decode_temperature(registers))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registers(time: [u8; 7]) -> [u8; CLOCK_REGISTERS] {
        let mut r = [0; CLOCK_REGISTERS];
        r[..7].copy_from_slice(&time);
        r
    }

    #[test]
    fn decodes_a_24_hour_time() {
        let reading = decode_clock(&registers([0x07, 0x42, 0x17, 5, 0x25, 0x09, 0x26]), false);
        assert_eq!(reading.time, DateTime { year: 2026, month: 9, day: 25, hour: 17, minute: 42, second: 7 });
        assert!(!reading.oscillator_stopped);
    }

    #[test]
    fn decodes_twelve_hour_mode() {
        let at = |h| decode_clock(&registers([0, 0, h, 1, 1, 1, 0]), false).time.hour;
        assert_eq!(at(TWELVE_HOUR_MODE | 0x12), 0);
        assert_eq!(at(TWELVE_HOUR_MODE | PM | 0x12), 12);
        assert_eq!(at(TWELVE_HOUR_MODE | PM | 0x07), 19);
    }

    #[test]
    fn decodes_a_stopped_oscillator() {
        let mut r = registers([0; 7]);
        r[STATUS] = OSCILLATOR_STOPPED;
        let reading = decode_clock(&r, true);
        assert!(reading.oscillator_stopped && reading.alarm_raised);
    }

    #[test]
    fn decodes_quarter_degrees_on_both_sides_of_zero() {
        assert_eq!(decode_temperature([24, 0xc0]).to_string(), "24.75 C");
        assert_eq!(decode_temperature([0xff, 0x40]).quarter_degrees_celsius, -3);
    }
}
