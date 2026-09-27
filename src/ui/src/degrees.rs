//! Temperatures as every screen writes them.

use domain::weather::Degrees;

pub(crate) fn temperature(degrees: Degrees) -> String {
    format!("{}°", degrees.0)
}
