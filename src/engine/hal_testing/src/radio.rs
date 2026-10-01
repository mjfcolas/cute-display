use hal::radio::WifiStation;
use hal::Fault;

/// Joins any network at once.
#[derive(Clone, Copy, Default)]
pub struct StubWifiStation;

impl WifiStation for StubWifiStation {
    fn connect(&mut self, _: &str, _: &str) -> Result<(), Fault> {
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}
