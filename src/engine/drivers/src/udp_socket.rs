//! UDP through the standard library's sockets, which ESP-IDF provides as well as any
//! computer.

use core::time::Duration;
use std::net::UdpSocket;

use hal::udp::UdpClient;
use hal::Fault;

#[derive(Default)]
pub struct StdUdpClient;

impl UdpClient for StdUdpClient {
    fn exchange(&mut self, host: &str, port: u16, request: &[u8], answer: &mut [u8], timeout: Duration) -> Result<usize, Fault> {
        let fault = |doing: &'static str| move |e: std::io::Error| Fault::new(format!("{doing} {host}: {e}"));
        let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(fault("opening a socket for"))?;
        socket.set_read_timeout(Some(timeout)).map_err(fault("setting the timeout for"))?;
        socket.connect((host, port)).map_err(fault("reaching"))?;
        socket.send(request).map_err(fault("sending to"))?;
        socket.recv(answer).map_err(fault("no answer from"))
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn the_answer_to_a_request_comes_back() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let echo = thread::spawn(move || {
            let mut buffer = [0u8; 16];
            let (length, from) = server.recv_from(&mut buffer).unwrap();
            server.send_to(&buffer[..length], from).unwrap();
        });
        let mut answer = [0u8; 16];
        let length = StdUdpClient.exchange("127.0.0.1", port, b"ping", &mut answer, Duration::from_secs(2)).unwrap();
        assert_eq!(&answer[..length], b"ping");
        echo.join().unwrap();
    }

    #[test]
    fn a_silent_server_is_a_fault_after_the_timeout() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let mut answer = [0u8; 16];
        assert!(StdUdpClient.exchange("127.0.0.1", port, b"ping", &mut answer, Duration::from_millis(50)).is_err());
    }
}
