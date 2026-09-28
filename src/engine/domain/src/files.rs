use crate::fetch::Unavailable;

pub trait Files: Send {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable>;
    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable>;
}
