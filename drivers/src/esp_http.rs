use core::time::Duration;
use std::io::{self, Read};

use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::sys::esp_crt_bundle_attach;
use hal::http::HttpClient;
use hal::Fault;

use crate::or_fault::OrFault;

const TIMEOUT: Duration = Duration::from_secs(15);

/// HTTP and HTTPS, trusting the certificate authorities ESP-IDF bundles.
pub struct EspHttpsClient;

struct Body<'a>(&'a mut EspHttpConnection);

impl Read for Body<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).map_err(io::Error::other)
    }
}

impl HttpClient for EspHttpsClient {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let configuration = Configuration {
            timeout: Some(TIMEOUT),
            crt_bundle_attach: Some(esp_crt_bundle_attach),
            ..Default::default()
        };
        let mut connection = EspHttpConnection::new(&configuration).or_fault("HTTP client")?;
        connection.initiate_request(Method::Get, url, &[]).or_fault("sending the request")?;
        connection.initiate_response().or_fault("waiting for the answer")?;
        let status = connection.status();
        if !(200..300).contains(&status) {
            return Err(Fault::new(format!("the server answered {status}")));
        }
        read(&mut Body(&mut connection))
    }
}
