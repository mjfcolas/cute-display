use crate::fetch::Unavailable;

/// An app's own files, by their names in its directory: `name` or `directory/name`.
pub trait Files: Send {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable>;
    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable>;
    /// At most `max_bytes` from `offset` on, fewer towards the end; `None` when there is
    /// no such file.
    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable>;
    /// The names of the files in one of the app's directories, in order; none when it has
    /// no such directory.
    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable>;
}
