use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use domain::fetch::Unavailable;
use domain::files::Files;

#[derive(Clone, Default)]
pub struct MemoryFiles(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl MemoryFiles {
    pub fn put(&self, name: &str, contents: &[u8]) {
        self.0.lock().unwrap().insert(name.into(), contents.to_vec());
    }
}

impl Files for MemoryFiles {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        Ok(self.0.lock().unwrap().get(name).map(|bytes| String::from_utf8_lossy(bytes).into_owned()))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        self.put(name, text.as_bytes());
        Ok(())
    }

    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        let files = self.0.lock().unwrap();
        Ok(files.get(name).map(|bytes| bytes.iter().skip(offset as usize).take(max_bytes).copied().collect()))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        let prefix = format!("{directory}/");
        let files = self.0.lock().unwrap();
        Ok(files.keys().filter_map(|name| name.strip_prefix(&prefix)).filter(|name| !name.contains('/')).map(String::from).collect())
    }
}
