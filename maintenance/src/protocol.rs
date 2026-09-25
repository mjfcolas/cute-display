//! The console's lines, both ways.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::path::SdPath;

pub const PREFIX: &str = "@@";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    List(SdPath),
    Get(SdPath),
    Put { path: SdPath, size_bytes: usize, crc32: u32 },
    Data(Vec<u8>),
    End,
    Remove(SdPath),
}

/// One line of the protocol: `None` for any other line.
pub fn parse(line: &str) -> Option<(String, Result<Request, String>)> {
    let mut words = line.trim().split(' ').filter(|w| !w.is_empty());
    if words.next() != Some(PREFIX) {
        return None;
    }
    let id = words.next()?.to_owned();
    let verb = words.next().unwrap_or_default();
    let args: Vec<&str> = words.collect();
    let path = |n: usize| SdPath::parse(args.get(n).copied().unwrap_or_default()).map_err(str::to_owned);
    let number = |n: usize| args.get(n).and_then(|a| a.parse::<u64>().ok()).ok_or(format!("{verb}: a number is missing"));

    let request = match verb {
        "ls" => path(0).map(Request::List),
        "get" => path(0).map(Request::Get),
        "rm" => path(0).map(Request::Remove),
        "put" => path(0).and_then(|path| {
            let size_bytes = number(1)? as usize;
            let crc32 = u32::try_from(number(2)?).map_err(|_| "put: the CRC is not 32 bits".to_owned())?;
            Ok(Request::Put { path, size_bytes, crc32 })
        }),
        "data" => STANDARD.decode(args.first().copied().unwrap_or_default()).map(Request::Data).map_err(|e| format!("data: {e}")),
        "end" => Ok(Request::End),
        other => Err(format!("unknown request '{other}'")),
    };
    Some((id, request))
}

pub fn ok(id: &str, detail: &str) -> String {
    if detail.is_empty() {
        format!("{PREFIX} {id} ok")
    } else {
        format!("{PREFIX} {id} ok {detail}")
    }
}

/// A put was accepted: its data may come.
pub fn ready(id: &str) -> String {
    format!("{PREFIX} {id} ready")
}

/// A data line was taken in: the next may come. The console drops what arrives faster
/// than it reads, so the computer waits for this.
pub fn ack(id: &str) -> String {
    format!("{PREFIX} {id} ack")
}

pub fn error(id: &str, reason: &str) -> String {
    format!("{PREFIX} {id} error {reason}")
}

pub fn entry(id: &str, name: &str, size_bytes: u64, is_dir: bool) -> String {
    format!("{PREFIX} {id} entry {} {size_bytes} {name}", if is_dir { 'd' } else { 'f' })
}

pub fn data(id: &str, bytes: &[u8]) -> String {
    format!("{PREFIX} {id} data {}", STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_that_are_not_the_protocols_are_none() {
        assert_eq!(parse("I (1787) app: cute-display, build 09-25"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("@@"), None);
    }

    #[test]
    fn requests_parse_with_their_id() {
        assert_eq!(parse("@@ 7 ls cute-display"), Some(("7".into(), Ok(Request::List(SdPath::parse("cute-display").unwrap())))));
        assert_eq!(parse("@@ 3 ls"), Some(("3".into(), Ok(Request::List(SdPath::parse("").unwrap())))));
        assert_eq!(
            parse("@@ 9 put cute-display/wifi.conf 12 305419896"),
            Some((
                "9".into(),
                Ok(Request::Put { path: SdPath::parse("cute-display/wifi.conf").unwrap(), size_bytes: 12, crc32: 0x1234_5678 })
            ))
        );
        assert_eq!(parse("@@ 9 data aGVsbG8="), Some(("9".into(), Ok(Request::Data(b"hello".to_vec())))));
    }

    #[test]
    fn a_bad_request_is_an_error_for_its_id() {
        for bad in ["@@ 4 frobnicate", "@@ 4 get ../x", "@@ 4 put cute-display/a nope 1", "@@ 4 data !!!"] {
            assert!(matches!(parse(bad), Some((id, Err(_))) if id == "4"), "{bad}");
        }
    }
}
