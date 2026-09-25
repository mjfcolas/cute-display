use crate::Fault;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockReading {
    pub time: DateTime,
    /// The oscillator stopped since the time was last set, so `time` is meaningless.
    pub oscillator_stopped: bool,
    /// An alarm fired and holds the interrupt line until acknowledged.
    pub alarm_raised: bool,
}

pub trait RealTimeClock {
    fn read(&mut self) -> Result<ClockReading, Fault>;
}
