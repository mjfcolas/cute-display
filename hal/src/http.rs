use crate::Fault;

pub trait HttpClient {
    /// The body of a successful answer; any other status is a fault.
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Fault>;
}
