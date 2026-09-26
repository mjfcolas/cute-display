use core::fmt;

/// A piece of hardware did not do what it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault(String);

impl Fault {
    pub fn new(reason: impl fmt::Display) -> Self {
        Self(reason.to_string())
    }

    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Fault {}
