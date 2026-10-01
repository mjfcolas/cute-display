//! HTTP responses kept in a directory, one file each: the URL on the first line, the body
//! after it. A file is named after the URL's host and path, and a hash of its query, too
//! long for a file name: recording a URL again replaces it, and the files can be told
//! apart.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use hal::http::HttpClient;
use hal::Fault;

use crate::network::HostHttpClient;

const EXTENSION: &str = "http";

/// Answers the URLs recorded in a directory, read once at the start; any other is a fault,
/// as an unreachable server is.
pub struct RecordedWeb {
    recordings: Vec<(String, Vec<u8>)>,
}

impl RecordedWeb {
    pub fn read(directory: &Path) -> Result<Self, Fault> {
        let fault = |e: std::io::Error| Fault::new(format!("{}: {e}", directory.display()));
        let mut recordings = Vec::new();
        for entry in fs::read_dir(directory).map_err(fault)? {
            let path = entry.map_err(fault)?.path();
            if path.extension().is_some_and(|extension| extension == EXTENSION) {
                let recording = fs::read(&path).map_err(fault)?;
                let (url, body) = parse(&recording).ok_or_else(|| Fault::new(format!("{}: no URL on the first line", path.display())))?;
                recordings.push((url, body.to_vec()));
            }
        }
        Ok(Self { recordings })
    }
}

impl HttpClient for RecordedWeb {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let (_, body) = self.recordings.iter().find(|(recorded, _)| recorded == url).ok_or_else(|| Fault::new(format!("{url}: nothing recorded")))?;
        read(&mut body.as_slice())
    }
}

/// The computer's web, each response kept in a directory before it is read.
pub struct RecordingWeb {
    web: HostHttpClient,
    directory: PathBuf,
}

impl RecordingWeb {
    pub fn in_directory(directory: PathBuf) -> Result<Self, Fault> {
        fs::create_dir_all(&directory).map_err(|e| Fault::new(format!("{}: {e}", directory.display())))?;
        Ok(Self { web: HostHttpClient::default(), directory })
    }
}

impl HttpClient for RecordingWeb {
    fn fetch(&mut self, url: &str, read: &mut dyn FnMut(&mut dyn Read) -> Result<(), Fault>) -> Result<(), Fault> {
        let mut body = Vec::new();
        self.web.fetch(url, &mut |response| response.read_to_end(&mut body).map(drop).map_err(Fault::new))?;
        match record(&self.directory, url, &body) {
            Ok(path) => log::info!("recorded {}", path.display()),
            Err(fault) => log::warn!("{fault}"),
        }
        read(&mut body.as_slice())
    }
}

fn record(directory: &Path, url: &str, body: &[u8]) -> Result<PathBuf, Fault> {
    let path = directory.join(file_name(url));
    fs::write(&path, [url.as_bytes(), b"\n", body].concat()).map_err(|e| Fault::new(format!("{}: {e}", path.display())))?;
    Ok(path)
}

fn parse(recording: &[u8]) -> Option<(String, &[u8])> {
    let end_of_url = recording.iter().position(|&byte| byte == b'\n')?;
    let url = core::str::from_utf8(recording.get(..end_of_url)?).ok()?.trim_end();
    (!url.is_empty()).then(|| (url.to_owned(), recording.get(end_of_url + 1..).unwrap_or_default()))
}

fn file_name(url: &str) -> String {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let (host_and_path, query) = without_scheme.split_once('?').unwrap_or((without_scheme, ""));
    let name: String = host_and_path.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    let name = name.trim_end_matches('_');
    match query {
        "" => format!("{name}.{EXTENSION}"),
        query => format!("{name}-{:016x}.{EXTENSION}", fnv1a(query)),
    }
}

/// FNV-1a, 64 bits: the same name for the same query on any computer, any Rust.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN_METEO: &str = "https://api.open-meteo.com/v1/forecast?latitude=48.8530&longitude=2.3499";

    fn directory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("cute-display-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn fetch(web: &mut impl HttpClient, url: &str) -> Result<Vec<u8>, Fault> {
        let mut body = Vec::new();
        web.fetch(url, &mut |read| read.read_to_end(&mut body).map(drop).map_err(Fault::new))?;
        Ok(body)
    }

    #[test]
    fn a_file_is_named_after_the_host_and_the_path_of_its_url() {
        assert_eq!(file_name(OPEN_METEO), format!("api.open-meteo.com_v1_forecast-{:016x}.http", fnv1a("latitude=48.8530&longitude=2.3499")));
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325, "FNV-1a's offset basis");
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c, "FNV-1a's published value");
        assert_eq!(file_name("https://opendata.adsb.fi/api/v3/lat/48.8530/lon/2.3499/dist/14"), "opendata.adsb.fi_api_v3_lat_48.8530_lon_2.3499_dist_14.http");
    }

    #[test]
    fn the_url_recorded_is_answered_with_its_body_and_any_other_is_a_fault() {
        let root = directory("recorded-web");
        record(&root, OPEN_METEO, b"{\"hourly\": {}}\n").unwrap();
        fs::write(root.join("notes.txt"), "not a recording").unwrap();
        let mut web = RecordedWeb::read(&root).unwrap();
        assert_eq!(fetch(&mut web, OPEN_METEO), Ok(b"{\"hourly\": {}}\n".to_vec()));
        assert!(fetch(&mut web, "https://api.open-meteo.com/v1/forecast?latitude=0").is_err());
    }

    #[test]
    fn recording_a_url_again_replaces_what_it_answers() {
        let root = directory("recorded-again");
        record(&root, OPEN_METEO, b"old").unwrap();
        record(&root, OPEN_METEO, b"new").unwrap();
        assert_eq!(fetch(&mut RecordedWeb::read(&root).unwrap(), OPEN_METEO), Ok(b"new".to_vec()));
    }

    #[test]
    fn urls_that_differ_by_their_query_alone_are_both_answered() {
        let root = directory("recorded-two-places");
        let elsewhere = "https://api.open-meteo.com/v1/forecast?latitude=43.2965&longitude=5.3698";
        record(&root, OPEN_METEO, b"Paris").unwrap();
        record(&root, elsewhere, b"Marseille").unwrap();
        let mut web = RecordedWeb::read(&root).unwrap();
        assert_eq!((fetch(&mut web, OPEN_METEO), fetch(&mut web, elsewhere)), (Ok(b"Paris".to_vec()), Ok(b"Marseille".to_vec())));
    }

    #[test]
    fn a_recording_without_a_url_is_refused() {
        let root = directory("recorded-web-without-url");
        fs::write(root.join("empty.http"), "\n{}").unwrap();
        assert!(RecordedWeb::read(&root).is_err());
    }

    #[test]
    fn a_directory_that_is_not_there_is_a_fault() {
        assert!(RecordedWeb::read(Path::new("/nowhere/recordings")).is_err());
    }
}
