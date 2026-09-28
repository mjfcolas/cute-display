//! The engine's own conf files, read from a storage.

use conf_text::ConfText;
use hal::storage::FileStorage;
use hal::Fault;

/// `None` when the file is not there.
pub(crate) fn read(storage: &impl FileStorage, path: &str) -> Result<Option<ConfText>, Fault> {
    Ok(storage.read(path)?.map(|bytes| ConfText::parse(&String::from_utf8_lossy(&bytes))))
}
