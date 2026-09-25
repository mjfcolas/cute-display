//! What the domain says about data it has to fetch from outside.

use std::fmt;

/// Why the outside world gave nothing, in words a person can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unavailable(pub String);

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How fresh fetched data is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchStatus {
    NeverFetched,
    Updating,
    UpToDate,
    NoPlace,
    Failed(Unavailable),
}
