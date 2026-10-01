use std::sync::{Arc, Mutex};

use domain::fetch::Unavailable;
use domain::files::Files;

/// An app's files, for the app and for the recording playing on the speaker's thread.
#[derive(Clone)]
pub struct SharedFiles(Arc<Mutex<Box<dyn Files>>>);

impl SharedFiles {
    pub fn new(files: Box<dyn Files>) -> Self {
        Self(Arc::new(Mutex::new(files)))
    }

    fn with<T>(&self, use_files: impl FnOnce(&dyn Files) -> T) -> T {
        use_files(self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref())
    }
}

impl Files for SharedFiles {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        self.with(|files| files.read(name))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        self.with(|files| files.write(name, text))
    }

    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        self.with(|files| files.read_bytes(name, offset, max_bytes))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        self.with(|files| files.names_in(directory))
    }
}
