use crate::Fault;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

/// Paths are relative to the root of the storage, which is `""`.
pub trait FileStorage {
    fn entries(&self, dir: &str) -> Result<Vec<Entry>, Fault>;
    fn capacity_bytes(&self) -> Result<u64, Fault>;
    /// `None` when there is no such file.
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault>;
    /// Creates the file and the directories above it, or replaces what it held.
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault>;
    /// Removing a file that is not there is not a fault.
    fn remove(&self, path: &str) -> Result<(), Fault>;
}
