use crate::Fault;

pub trait PowerMonitor {
    fn on_external_power(&self) -> bool;

    /// The voltage at the battery sense input, before the divider is taken into account.
    fn battery_sense_millivolts(&mut self) -> Result<u32, Fault>;
}
