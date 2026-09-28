//! Files: text, by name, in a directory of their own.

use crate::fetch::Unavailable;

pub trait Files: Send {
    /// `None` when the file is not there.
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable>;
    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable>;
}
