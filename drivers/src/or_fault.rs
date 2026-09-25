use core::fmt::Display;

use hal::Fault;

pub(crate) trait OrFault<T> {
    fn or_fault(self, doing: &str) -> Result<T, Fault>;
}

impl<T, E: Display> OrFault<T> for Result<T, E> {
    fn or_fault(self, doing: &str) -> Result<T, Fault> {
        self.map_err(|e| Fault::new(format!("{doing}: {e}")))
    }
}
