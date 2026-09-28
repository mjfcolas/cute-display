//! Files: text, by name. The engine lends an app those it named, and no other.

use crate::fetch::Unavailable;

pub trait Files: Send {
    /// `None` when the file is not there.
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable>;
    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable>;
}
