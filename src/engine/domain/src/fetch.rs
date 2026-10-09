use std::fmt;

/// Why nothing came from the outside world, in words that tell a person what to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unavailable(pub String);

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchStatus {
    NeverFetched,
    Updating,
    UpToDate,
    NoPlace,
    Failed(Unavailable),
}
