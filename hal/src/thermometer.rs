use core::fmt;

use crate::Fault;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Temperature {
    pub quarter_degrees_celsius: i16,
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q = self.quarter_degrees_celsius;
        let sign = if q < 0 { "-" } else { "" };
        let q = q.unsigned_abs();
        write!(f, "{sign}{}.{:02} C", q / 4, q % 4 * 25)
    }
}

pub trait Thermometer {
    fn temperature(&mut self) -> Result<Temperature, Fault>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_reads_in_hundredths() {
        let t = |q| Temperature { quarter_degrees_celsius: q }.to_string();
        assert_eq!(t(99), "24.75 C");
        assert_eq!(t(0), "0.00 C");
        assert_eq!(t(-3), "-0.75 C");
    }
}
