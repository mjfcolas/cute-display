use crate::Fault;

/// Paths are relative to the root of the storage.
pub trait FileStorage {
    fn root_entries(&self) -> Result<Vec<String>, Fault>;
    fn capacity_bytes(&self) -> Result<u64, Fault>;
    /// `None` when there is no such file.
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault>;
    /// Creates the file, or replaces what it held.
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault>;
}
