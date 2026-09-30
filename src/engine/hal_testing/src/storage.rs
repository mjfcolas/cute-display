use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use hal::storage::{CopyOutcome, Entry, FileStorage};
use hal::Fault;

use crate::shared::lock;

/// Files in memory. A directory is there while a file is under it.
#[derive(Clone, Default)]
pub struct FakeFileStorage(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl FakeFileStorage {
    pub fn with(path: &str, contents: impl AsRef<[u8]>) -> Self {
        let storage = Self::default();
        storage.put(path, contents);
        storage
    }

    pub fn put(&self, path: &str, contents: impl AsRef<[u8]>) {
        lock(&self.0).insert(path.into(), contents.as_ref().to_vec());
    }

    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        lock(&self.0).get(path).cloned()
    }

    pub fn contains(&self, path: &str) -> bool {
        lock(&self.0).contains_key(path)
    }
}

impl FileStorage for FakeFileStorage {
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
        let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
        let mut entries: Vec<Entry> = Vec::new();
        for (path, contents) in lock(&self.0).iter() {
            let Some(below) = path.strip_prefix(&prefix) else { continue };
            let entry = match below.split_once('/') {
                Some((subdirectory, _)) => Entry { name: subdirectory.into(), size_bytes: 0, is_dir: true },
                None => Entry { name: below.into(), size_bytes: contents.len() as u64, is_dir: false },
            };
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
        Ok((dir.is_empty() || !entries.is_empty()).then_some(entries))
    }

    fn capacity_bytes(&self) -> Result<u64, Fault> {
        Err(Fault::new("memory has no capacity of its own"))
    }

    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        Ok(self.file(path))
    }

    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
        self.put(path, contents);
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), Fault> {
        lock(&self.0).remove(path);
        Ok(())
    }
}

/// Checks a storage, blank at first, against `FileStorage`'s contract.
pub fn check_contract(storage: &impl FileStorage) {
    const CONF: &str = "cute-display/apps/alarm/alarm.conf";
    const COPY: &str = "cute-display/apps/alarm/kept/alarm.conf";
    let file = |name: &str, size_bytes: u64| Entry { name: name.into(), size_bytes, is_dir: false };
    let dir = |name: &str| Entry { name: name.into(), size_bytes: 0, is_dir: true };

    assert_eq!(storage.read(CONF), Ok(None), "a file that is not there");
    assert_eq!(storage.read_range(CONF, 0, 4), Ok(None), "a range of a file that is not there");
    assert_eq!(storage.entries("cute-display"), Ok(None), "a directory that is not there");
    assert_eq!(storage.remove(CONF), Ok(()), "removing a file that is not there");

    assert_eq!(storage.write(CONF, b"on = no\n"), Ok(()));
    assert_eq!(storage.write(CONF, b"on = yes\n"), Ok(()), "writing again replaces");
    assert_eq!(storage.read(CONF), Ok(Some(b"on = yes\n".to_vec())));
    assert_eq!(storage.read_range(CONF, 5, 3), Ok(Some(b"yes".to_vec())));
    assert_eq!(storage.read_range(CONF, 5, 100), Ok(Some(b"yes\n".to_vec())), "a range stops at the end");
    assert_eq!(storage.read_range(CONF, 100, 4), Ok(Some(Vec::new())), "a range past the end");

    assert_eq!(sorted(storage.entries("")), Ok(Some(vec![dir("cute-display")])), "the root");
    assert_eq!(sorted(storage.entries("cute-display")), Ok(Some(vec![dir("apps")])), "the directories above are made");
    assert_eq!(sorted(storage.entries("cute-display/apps/alarm")), Ok(Some(vec![file("alarm.conf", 9)])));

    assert_eq!(storage.copy(CONF, COPY), Ok(CopyOutcome::Copied));
    assert_eq!(storage.read(COPY), Ok(Some(b"on = yes\n".to_vec())), "a copy is made under new directories");
    assert_eq!(storage.copy("cute-display/none.conf", COPY), Ok(CopyOutcome::NoSource));
    assert_eq!(storage.read(COPY), Ok(Some(b"on = yes\n".to_vec())), "a copy without a source leaves the destination");

    assert_eq!(storage.remove(CONF), Ok(()));
    assert_eq!(storage.read(CONF), Ok(None), "removed");
    assert_eq!(storage.read(COPY), Ok(Some(b"on = yes\n".to_vec())), "the copy stays");
}

fn sorted(entries: Result<Option<Vec<Entry>>, Fault>) -> Result<Option<Vec<Entry>>, Fault> {
    entries.map(|entries| {
        entries.map(|mut entries| {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            entries
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_file_storage_keeps_the_contract() {
        check_contract(&FakeFileStorage::default());
    }

    #[test]
    fn clones_share_the_files() {
        let storage = FakeFileStorage::with("cute-display/wifi.conf", "ssid = Home\n");
        let clone = storage.clone();
        clone.write("cute-display/general.conf", b"").unwrap();
        assert!(storage.contains("cute-display/general.conf"));
        assert_eq!(clone.file("cute-display/wifi.conf").as_deref(), Some(&b"ssid = Home\n"[..]));
    }
}
