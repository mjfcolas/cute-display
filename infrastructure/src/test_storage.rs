use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use hal::storage::{Entry, FileStorage};
use hal::Fault;

/// Files in memory; every clone sees the same ones.
#[derive(Clone, Default)]
pub(crate) struct MemoryStorage(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl MemoryStorage {
    pub fn with(path: &str, contents: &str) -> Self {
        let storage = Self::default();
        storage.0.lock().unwrap().insert(path.into(), contents.as_bytes().to_vec());
        storage
    }

    pub fn contains(&self, path: &str) -> bool {
        self.0.lock().unwrap().contains_key(path)
    }
}

impl FileStorage for MemoryStorage {
    fn entries(&self, _: &str) -> Result<Vec<Entry>, Fault> {
        Ok(vec![])
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
