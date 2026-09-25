use core::time::Duration;

use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::sys::esp_crt_bundle_attach;
use hal::http::HttpClient;
use hal::Fault;

use crate::or_fault::OrFault;

const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BODY_BYTES: usize = 32 * 1024;

/// HTTP and HTTPS, trusting the certificate authorities ESP-IDF bundles.
pub struct EspHttpsClient;

impl HttpClient for EspHttpsClient {
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Fault> {
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

        let mut body = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let read = connection.read(&mut chunk).or_fault("reading the answer")?;
            if read == 0 {
                return Ok(body);
            }
            body.extend_from_slice(chunk.get(..read).unwrap_or_default());
            if body.len() > MAX_BODY_BYTES {
                return Err(Fault::new(format!("the answer is larger than {MAX_BODY_BYTES} bytes")));
            }
        }
    }
}
