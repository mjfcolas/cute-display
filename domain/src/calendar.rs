#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Date {
    /// Gregorian calendar (Sakamoto's method).
    pub fn weekday(self) -> Weekday {
        const MONTH_OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let month = usize::from(self.month.clamp(1, 12));
        let year = i32::from(self.year) - i32::from(month < 3);
        let offset = MONTH_OFFSETS.get(month - 1).copied().unwrap_or(0);
        let from_sunday = (year + year / 4 - year / 100 + year / 400 + offset + i32::from(self.day)).rem_euclid(7);
        match from_sunday {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weekday(year: u16, month: u8, day: u8) -> Weekday {
        Date { year, month, day }.weekday()
    }

    #[test]
    fn known_days_fall_on_their_weekday() {
        assert_eq!(weekday(2026, 9, 25), Weekday::Friday);
        assert_eq!(weekday(2000, 1, 1), Weekday::Saturday);
        assert_eq!(weekday(2024, 2, 29), Weekday::Thursday);
        assert_eq!(weekday(2025, 12, 31), Weekday::Wednesday);
        assert_eq!(weekday(2027, 3, 1), Weekday::Monday);
    }
}
