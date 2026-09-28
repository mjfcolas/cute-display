use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use hal::storage::{Entry, FileStorage};
use hal::Fault;

#[derive(Clone, Default)]
pub(crate) struct MemoryStorage(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl MemoryStorage {
    pub fn with(path: &str, contents: &str) -> Self {
        let storage = Self::default();
        storage.put(path, contents.as_bytes());
        storage
    }

    pub fn put(&self, path: &str, contents: &[u8]) {
        self.0.lock().unwrap().insert(path.into(), contents.to_vec());
    }

    pub fn contains(&self, path: &str) -> bool {
        self.0.lock().unwrap().contains_key(path)
    }
}

impl FileStorage for MemoryStorage {
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
        let prefix = format!("{dir}/");
        let files = self.0.lock().unwrap();
        let mut entries: Vec<Entry> = Vec::new();
        for (path, contents) in files.iter() {
            let Some(below) = path.strip_prefix(&prefix) else { continue };
            let entry = match below.split_once('/') {
                Some((subdirectory, _)) => Entry { name: subdirectory.into(), size_bytes: 0, is_dir: true },
                None => Entry { name: below.into(), size_bytes: contents.len() as u64, is_dir: false },
            };
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
        Ok((!entries.is_empty()).then_some(entries))
    }
    fn capacity_bytes(&self) -> Result<u64, Fault> {
        Ok(0)
    }
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        Ok(self.0.lock().unwrap().get(path).cloned())
    }
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
        self.0.lock().unwrap().insert(path.into(), contents.to_vec());
        Ok(())
    }
    fn remove(&self, path: &str) -> Result<(), Fault> {
        self.0.lock().unwrap().remove(path);
        Ok(())
    }
}
