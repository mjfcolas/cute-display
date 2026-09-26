//! Days in the Gregorian calendar.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
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

impl Weekday {
    /// Monday first.
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// 0 for Sunday, 1 for Monday, and so on to 6 for Saturday.
    pub fn days_since_sunday(self) -> u8 {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }

    /// 0 for Monday, and so on to 6 for Sunday: where it is in [`Weekday::ALL`].
    pub fn days_since_monday(self) -> usize {
        usize::from((self.days_since_sunday() + 6) % 7)
    }

    /// `None` above 6.
    pub fn from_days_since_sunday(days: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|day| day.days_since_sunday() == days)
    }

    pub fn next(self) -> Self {
        Self::from_days_since_sunday((self.days_since_sunday() + 1) % 7).unwrap_or(Self::Monday)
    }
}

impl Date {
    /// `None` for a month outside 1 to 12 or a day the month does not have.
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)).then_some(Self { year, month, day })
    }

    pub fn year(self) -> u16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// Days since 1970-01-01, negative before (Howard Hinnant's algorithm).
    pub fn days_since_epoch(self) -> i64 {
        let (month, day) = (i64::from(self.month), i64::from(self.day));
        let year = i64::from(self.year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `days` after 1970-01-01, held between 0000-03-01 and 65535-12-31.
    pub fn from_days_since_epoch(days: i64) -> Self {
        let days = days.clamp(-719_468, 23_217_003) + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days.rem_euclid(146_097);
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self {
            year: u16::try_from(year).unwrap_or(u16::MAX),
            month: u8::try_from(month).unwrap_or(1),
            day: u8::try_from(day).unwrap_or(1),
        }
    }

    /// The date `days` later, or earlier when negative.
    pub fn plus_days(self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + days)
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        let days_since_sunday = (self.days_since_epoch() + 4).rem_euclid(7);
        Weekday::from_days_since_sunday(u8::try_from(days_since_sunday).unwrap_or(0)).unwrap_or(Weekday::Sunday)
    }
}

pub fn days_in_month(year: u16, month: u8) -> u8 {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    fn weekday(year: u16, month: u8, day: u8) -> Weekday {
        date(year, month, day).weekday()
    }

    #[test]
    fn known_days_fall_on_their_weekday() {
        assert_eq!(weekday(2026, 9, 25), Weekday::Friday);
        assert_eq!(weekday(2000, 1, 1), Weekday::Saturday);
        assert_eq!(weekday(2024, 2, 29), Weekday::Thursday);
        assert_eq!(weekday(2025, 12, 31), Weekday::Wednesday);
        assert_eq!(weekday(2027, 3, 1), Weekday::Monday);
        assert_eq!(weekday(1969, 12, 31), Weekday::Wednesday);
    }

    #[test]
    fn a_month_or_a_day_out_of_range_is_no_date() {
        assert_eq!(Date::new(2026, 0, 1), None);
        assert_eq!(Date::new(2026, 13, 1), None);
        assert_eq!(Date::new(2026, 1, 0), None);
        assert_eq!(Date::new(2026, 1, 32), None);
        assert_eq!(Date::new(2026, 4, 31), None);
        assert_eq!(Date::new(2026, 2, 29), None);
        assert!(Date::new(2024, 2, 29).is_some());
        assert_eq!(Date::new(1900, 2, 29), None);
        assert!(Date::new(2000, 2, 29).is_some());
    }

    #[test]
    fn days_count_from_the_epoch_both_ways() {
        assert_eq!(date(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(date(2026, 9, 26).days_since_epoch(), 20_722);
        assert_eq!(date(1969, 12, 31).days_since_epoch(), -1);
        let start = date(2023, 12, 25);
        for n in 0..800 {
            let later = start.plus_days(n);
            assert_eq!(later.days_since_epoch(), start.days_since_epoch() + n);
            assert_eq!(Date::new(later.year(), later.month(), later.day()), Some(later));
        }
        assert_eq!(date(2024, 2, 28).plus_days(1), date(2024, 2, 29));
        assert_eq!(date(2026, 1, 1).plus_days(-1), date(2025, 12, 31));
    }

    #[test]
    fn a_weekday_is_where_it_stands_in_the_week() {
        for (n, day) in Weekday::ALL.iter().enumerate() {
            assert_eq!(day.days_since_monday(), n);
        }
    }

    #[test]
    fn weekdays_follow_one_another_round_the_week() {
        let mut day = Weekday::Monday;
        for expected in Weekday::ALL.iter().cycle().skip(1).take(7) {
            day = day.next();
            assert_eq!(day, *expected);
        }
    }
}
