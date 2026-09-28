use crate::Fault;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Brightness(u8);

impl Brightness {
    pub const OFF: Self = Self(0);
    pub const FULL: Self = Self(100);

    pub const fn percent(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    pub fn as_percent(self) -> u8 {
        self.0
    }

    pub fn adjusted_by(self, percent: i32) -> Self {
        Self((i32::from(self.0) + percent).clamp(0, 100) as u8)
    }
}

pub trait DimmableLight {
    fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault>;
    fn brightness(&self) -> Brightness;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_stays_between_off_and_full() {
        assert_eq!(Brightness::percent(250), Brightness::FULL);
        assert_eq!(Brightness::percent(95).adjusted_by(10), Brightness::FULL);
        assert_eq!(Brightness::percent(5).adjusted_by(-10), Brightness::OFF);
        assert_eq!(Brightness::percent(40).adjusted_by(-10).as_percent(), 30);
    }
}
