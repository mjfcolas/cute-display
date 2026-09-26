use crate::Fault;

pub trait PowerMonitor {
    fn on_external_power(&self) -> bool;

    /// Voltage at the battery sense input, before the divider is accounted for.
    fn battery_sense_millivolts(&mut self) -> Result<u32, Fault>;
}
