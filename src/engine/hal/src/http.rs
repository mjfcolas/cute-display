use std::io::Read;

use crate::Fault;

pub trait HttpClient {
    /// A status other than success is a fault.
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault>;
}
