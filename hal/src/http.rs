use std::io::Read;

use crate::Fault;

pub trait HttpClient {
    /// Hands the body of a successful answer to `read` as it arrives, so it never has to
    /// be held whole; any other status is a fault.
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault>;
}
