use core::ffi::CStr;
use std::io::Read;

use esp_idf_svc::http::client::Configuration;
use esp_idf_svc::tls::X509;
use hal::http::HttpClient;
use hal::Fault;

use crate::esp_http::{fetch_with, TIMEOUT};

/// The test image reaches its stubs, and nothing on the Internet.
pub struct EspHttpsClientTrustingOneAuthority {
    authority_pem: &'static CStr,
}

impl EspHttpsClientTrustingOneAuthority {
    pub fn new(authority_pem: &'static CStr) -> Self {
        Self { authority_pem }
    }
}

impl HttpClient for EspHttpsClientTrustingOneAuthority {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        // ESP-IDF calls it the server's certificate. esp-tls checks the server's chain
        // against it, as an authority, and checks the server's name against the URL's host.
        let configuration = Configuration {
            timeout: Some(TIMEOUT),
            server_certificate: Some(X509::pem(self.authority_pem)),
            ..Default::default()
        };
        fetch_with(&configuration, url, read)
    }
}
