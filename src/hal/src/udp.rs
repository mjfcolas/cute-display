use core::time::Duration;

use crate::Fault;

/// Datagrams on the network the device is on.
pub trait UdpClient {
    /// Sends `request` to `host:port` and waits up to `timeout` for the answer, which it
    /// puts in `answer`; returns its length.
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8], timeout: Duration) -> Result<usize, Fault>;
}
