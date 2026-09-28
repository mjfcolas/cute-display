//! Every time zone the installer offers, as the POSIX rule it writes into general.conf,
//! reads on the device. `tzdata_rules.txt` is the installer's list: its
//! `tests/test_config_files.py` says when it no longer matches the tzdata it ships.

use domain::time_zone::TimeZone;

#[test]
fn every_rule_the_installer_writes_parses() {
    let refused: Vec<&str> = include_str!("tzdata_rules.txt")
        .lines()
        .filter(|line| line.split_once(' ').is_none_or(|(_, rule)| TimeZone::parse(rule).is_none()))
        .collect();
    assert!(refused.is_empty(), "{} rules refused: {refused:?}", refused.len());
}
