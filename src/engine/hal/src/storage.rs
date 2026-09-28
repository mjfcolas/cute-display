use crate::Fault;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyOutcome {
    Copied,
    NoSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

/// Paths are relative to the root of the storage, which is `""`.
pub trait FileStorage {
    /// `None` when there is no such directory.
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault>;
    fn capacity_bytes(&self) -> Result<u64, Fault>;
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault>;
    fn read_range(&self, path: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Fault> {
        Ok(self.read(path)?.map(|contents| {
            let start = usize::try_from(offset).unwrap_or(usize::MAX).min(contents.len());
            contents.get(start..).unwrap_or_default().iter().take(max_bytes).copied().collect()
        }))
    }
    /// Creates the file and the directories above it, or replaces what it held.
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault>;
    /// Creates `to` and the directories above it, or replaces what it held.
    fn copy(&self, from: &str, to: &str) -> Result<CopyOutcome, Fault> {
        let Some(contents) = self.read(from)? else {
            return Ok(CopyOutcome::NoSource);
        };
        self.write(to, &contents).map(|()| CopyOutcome::Copied)
    }
    /// Removing a file that is not there is not a fault.
    fn remove(&self, path: &str) -> Result<(), Fault>;
}
