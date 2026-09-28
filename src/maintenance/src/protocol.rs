use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use hal::storage::Entry;

use crate::path::SdPath;

pub const PREFIX: &str = "@@";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    List(SdPath),
    Get { path: SdPath, offset: u64 },
    Put { path: SdPath, size_bytes: usize, crc32: u32 },
    Data(Vec<u8>),
    End,
    Remove(SdPath),
    Copy { from: SdPath, to: SdPath },
}

/// A path comes last and runs to the end of the line, since names on the card have spaces.
/// The two paths of a copy are apart by a tab, which no name on a FAT card holds.
pub fn parse(line: &str) -> Option<(String, Result<Request, String>)> {
    let rest = line.trim_end_matches(['\r', '\n']).strip_prefix(PREFIX)?.strip_prefix(' ')?;
    let (id, rest) = first_word(rest);
    if id.is_empty() {
        return None;
    }
    let (verb, rest) = first_word(rest);
    let path = |text: &str| SdPath::parse(text).map_err(str::to_owned);
    let number = |word: &str| word.parse::<u64>().map_err(|_| format!("{verb}: a number is missing"));

    let request = match verb {
        "ls" => path(rest).map(Request::List),
        "rm" => path(rest).map(Request::Remove),
        "get" => {
            let (offset, rest) = first_word(rest);
            number(offset).and_then(|offset| Ok(Request::Get { path: path(rest)?, offset }))
        }
        "put" => {
            let (size_bytes, rest) = first_word(rest);
            let (crc32, rest) = first_word(rest);
            (|| {
                let size_bytes = usize::try_from(number(size_bytes)?).map_err(|_| "put: too large".to_owned())?;
                let crc32 = u32::try_from(number(crc32)?).map_err(|_| "put: the CRC is not 32 bits".to_owned())?;
                Ok(Request::Put { path: path(rest)?, size_bytes, crc32 })
            })()
        }
        "data" => STANDARD.decode(rest).map(Request::Data).map_err(|e| format!("data: {e}")),
        "end" => Ok(Request::End),
        "cp" => match rest.split_once('\t') {
            Some((from, to)) => path(from).and_then(|from| Ok(Request::Copy { from, to: path(to)? })),
            None => Err("cp: the two paths are apart by a tab".to_owned()),
        },
        other => Err(format!("unknown request '{other}'")),
    };
    Some((id.to_owned(), request))
}

fn first_word(text: &str) -> (&str, &str) {
    text.split_once(' ').unwrap_or((text, ""))
}

pub fn ok(id: &str, detail: &str) -> String {
    if detail.is_empty() {
        format!("{PREFIX} {id} ok")
    } else {
        format!("{PREFIX} {id} ok {detail}")
    }
}

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

pub fn entry(id: &str, entry: &Entry) -> String {
    format!("{PREFIX} {id} entry {} {} {}", if entry.is_dir { 'd' } else { 'f' }, entry.size_bytes, entry.name)
}

pub fn data(id: &str, bytes: &[u8]) -> String {
    format!("{PREFIX} {id} data {}", STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_that_are_not_the_protocols_are_none() {
        assert_eq!(parse("I (1787) app: Cute Display 2026.9.0, build 09-25"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("@@"), None);
    }

    #[test]
    fn requests_parse_with_their_id() {
        assert_eq!(parse("@@ 7 ls cute-display"), Some(("7".into(), Ok(Request::List(SdPath::parse("cute-display").unwrap())))));
        assert_eq!(parse("@@ 3 ls"), Some(("3".into(), Ok(Request::List(SdPath::parse("").unwrap())))));
        assert_eq!(
            parse("@@ 9 put 12 305419896 cute-display/wifi.conf"),
            Some((
                "9".into(),
                Ok(Request::Put { path: SdPath::parse("cute-display/wifi.conf").unwrap(), size_bytes: 12, crc32: 0x1234_5678 })
            ))
        );
        assert_eq!(parse("@@ 9 data aGVsbG8="), Some(("9".into(), Ok(Request::Data(b"hello".to_vec())))));
    }

    #[test]
    fn a_path_runs_to_the_end_of_the_line() {
        let path = SdPath::parse("sounds/alarm/Lost Ark.mp3").unwrap();
        assert_eq!(parse("@@ 2 get 4096 sounds/alarm/Lost Ark.mp3"), Some(("2".into(), Ok(Request::Get { path, offset: 4096 }))));
    }

    #[test]
    fn a_copy_names_both_paths_apart_by_a_tab() {
        let from = SdPath::parse("sounds/alarm/Lost Ark.mp3").unwrap();
        let to = SdPath::parse("cute-display/apps/alarm/ringtones/Lost Ark.mp3").unwrap();
        assert_eq!(
            parse("@@ 6 cp sounds/alarm/Lost Ark.mp3\tcute-display/apps/alarm/ringtones/Lost Ark.mp3"),
            Some(("6".into(), Ok(Request::Copy { from, to })))
        );
    }

    #[test]
    fn a_bad_request_is_an_error_for_its_id() {
        let bad_requests =
            ["@@ 4 frobnicate", "@@ 4 get 0 ../x", "@@ 4 get x", "@@ 4 put nope 1 cute-display/a", "@@ 4 data !!!", "@@ 4 cp a b", "@@ 4 cp a\t../b"];
        for bad in bad_requests {
            assert!(matches!(parse(bad), Some((id, Err(_))) if id == "4"), "{bad}");
        }
    }
}
