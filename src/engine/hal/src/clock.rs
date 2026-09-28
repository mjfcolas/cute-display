use crate::Fault;

const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    pub fn unix_seconds(self) -> i64 {
        let days = days_from_civil(i64::from(self.year), i64::from(self.month), i64::from(self.day));
        days * SECONDS_PER_DAY + i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second)
    }

    pub fn from_unix_seconds(seconds: i64) -> Self {
        let (days, second_of_day) = (seconds.div_euclid(SECONDS_PER_DAY), seconds.rem_euclid(SECONDS_PER_DAY));
        let (year, month, day) = civil_from_days(days);
        let narrow = |n: i64| u8::try_from(n).unwrap_or(0);
        Self {
            year: u16::try_from(year.clamp(0, i64::from(u16::MAX))).unwrap_or(0),
            month: narrow(month),
            day: narrow(day),
            hour: narrow(second_of_day / 3600),
            minute: narrow(second_of_day / 60 % 60),
            second: narrow(second_of_day % 60),
        }
    }
}

/// Howard Hinnant's algorithm.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockReading {
    pub time: DateTime,
    /// The oscillator stopped since the time was last set, so `time` is meaningless.
    pub oscillator_stopped: bool,
    /// An alarm fired and holds the interrupt line until acknowledged.
    pub alarm_raised: bool,
}

pub trait RealTimeClock {
    fn read(&mut self) -> Result<ClockReading, Fault>;
    /// From then on the time counts from `time`, and the oscillator is no longer stopped.
    fn set(&mut self, time: DateTime) -> Result<(), Fault>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> DateTime {
        DateTime { year, month, day, hour, minute, second }
    }

    #[test]
    fn known_instants_are_their_unix_seconds() {
        assert_eq!(at(1970, 1, 1, 0, 0, 0).unix_seconds(), 0);
        assert_eq!(at(2000, 3, 1, 0, 0, 0).unix_seconds(), 951_868_800);
        assert_eq!(at(2026, 9, 26, 7, 30, 15).unix_seconds(), 1_790_407_815);
        assert_eq!(at(1969, 12, 31, 23, 59, 59).unix_seconds(), -1);
    }

    #[test]
    fn every_day_of_a_leap_year_and_its_neighbours_comes_back() {
        let start = at(2023, 12, 30, 23, 59, 58).unix_seconds();
        for day in 0..370 {
            let seconds = start + day * SECONDS_PER_DAY;
            assert_eq!(DateTime::from_unix_seconds(seconds).unix_seconds(), seconds);
        }
        assert_eq!(DateTime::from_unix_seconds(at(2024, 2, 29, 12, 0, 0).unix_seconds()), at(2024, 2, 29, 12, 0, 0));
    }
}
