use domain::calendar::Weekday;

pub fn weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

pub fn short_weekday(day: Weekday) -> &'static str {
    weekday(day).get(..3).unwrap_or_default()
}

pub fn month(month: u8) -> Option<&'static str> {
    const MONTHS: [&str; 12] =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    MONTHS.get(usize::from(month).checked_sub(1)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_are_three_letters() {
        assert_eq!(short_weekday(Weekday::Wednesday), "Wed");
        assert_eq!(month(9), Some("September"));
        assert_eq!(month(0), None);
        assert_eq!(month(13), None);
    }
}
