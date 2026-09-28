//! An app's files on the card, in the engine's directory, `cute-display/`: only those
//! the app named when it was installed.

use domain::fetch::Unavailable;
use domain::files::Files;
use hal::storage::FileStorage;
use hal::Fault;

const DIRECTORY: &str = "cute-display";

pub struct CardFiles<S> {
    storage: S,
    names: &'static [&'static str],
}

impl<S: FileStorage> CardFiles<S> {
    pub fn new(storage: S, names: &'static [&'static str]) -> Self {
        Self { storage, names }
    }

    fn path(&self, name: &str) -> Result<String, Unavailable> {
        if self.names.contains(&name) {
            Ok(format!("{DIRECTORY}/{name}"))
        } else {
            Err(Unavailable(format!("{name} is not one of this app's files")))
        }
    }
}

impl<S: FileStorage + Send> Files for CardFiles<S> {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        let path = self.path(name)?;
        let bytes = self.storage.read(&path).map_err(|fault| Unavailable(format!("{path}: {fault}")))?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        let path = self.path(name)?;
        self.storage.write(&path, text.as_bytes()).map_err(|fault| Unavailable(format!("{path}: {fault}")))
    }
}

/// An app's files when the card could not be reached: each read and write says why.
pub struct UnreachableCard(pub Fault);

impl Files for UnreachableCard {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        Err(Unavailable(format!("{name}: no SD card ({})", self.0)))
    }

    fn write(&self, name: &str, _: &str) -> Result<(), Unavailable> {
        Err(Unavailable(format!("{name}: no SD card ({})", self.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    #[test]
    fn an_app_reads_and_writes_its_files_in_the_engines_directory() {
        let storage = MemoryStorage::with("cute-display/alarm.conf", "enabled = yes\n");
        let files = CardFiles::new(storage.clone(), &["alarm.conf", "notes.txt"]);
        assert_eq!(files.read("alarm.conf").unwrap().as_deref(), Some("enabled = yes\n"));
        files.write("notes.txt", "hello").unwrap();
        assert!(storage.contains("cute-display/notes.txt"));
    }

    #[test]
    fn a_file_the_app_did_not_name_is_out_of_its_reach() {
        let storage = MemoryStorage::with("cute-display/wifi.conf", "password = secret\n");
        let files = CardFiles::new(storage.clone(), &["alarm.conf"]);
        assert!(files.read("wifi.conf").is_err());
        assert!(files.write("../sounds/a.wav", "").is_err());
        assert!(!storage.contains("sounds/a.wav"));
    }

    #[test]
    fn without_the_card_an_app_is_told_why_rather_than_its_files_missing() {
        let files = UnreachableCard(Fault::new("not mounted"));
        let unread = files.read("alarm.conf").unwrap_err();
        assert!(unread.0.contains("not mounted"), "{unread}");
        assert!(files.write("alarm.conf", "enabled = yes").is_err());
    }
}
