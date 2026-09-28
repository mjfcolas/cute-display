use core::time::Duration;

use crate::Fault;

pub trait UdpClient {
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8], timeout: Duration) -> Result<usize, Fault>;
}
