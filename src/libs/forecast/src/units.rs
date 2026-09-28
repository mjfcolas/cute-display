//! The weather's quantities as every screen writes them.

use crate::{Degrees, Hectopascals, KilometresPerHour, Percent};

pub fn temperature(degrees: Degrees) -> String {
    format!("{}°", degrees.0)
}

pub fn percent(percent: Percent) -> String {
    format!("{}%", percent.value())
}

pub fn pressure(pressure: Hectopascals) -> String {
    format!("{} hPa", pressure.0)
}

/// Its speed; where it blows from is drawn.
pub fn wind_speed(speed: KilometresPerHour) -> String {
    if speed.0 == 0 {
        return "calm".into();
    }
    format!("{} km/h", speed.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_are_written_short() {
        assert_eq!(temperature(Degrees(-3)), "-3°");
        assert_eq!(percent(Percent::saturating(40)), "40%");
        assert_eq!(pressure(Hectopascals(1016)), "1016 hPa");
        assert_eq!(wind_speed(KilometresPerHour(12)), "12 km/h");
        assert_eq!(wind_speed(KilometresPerHour(0)), "calm");
    }
}
