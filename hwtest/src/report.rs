//! What the bench found, one check per line of the screen.

use hal::clock::ClockReading;
use hal::display::Refreshed;
use hal::light::Brightness;
use hal::thermometer::Temperature;
use hal::Fault;

use crate::survey::Survey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    /// Not exercised yet.
    Pending,
    /// A value to read, neither good nor bad.
    Reading,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub subject: &'static str,
    pub verdict: Verdict,
    pub reading: String,
}

impl Check {
    fn new(subject: &'static str, verdict: Verdict, reading: impl Into<String>) -> Self {
        Self { subject, verdict, reading: reading.into() }
    }

    fn fault(subject: &'static str, fault: &Fault) -> Self {
        Self::new(subject, Verdict::Fail, fault.reason())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub hardware: Vec<Check>,
    pub controls: Vec<Check>,
}

pub fn panel(last: &Option<Result<Refreshed, Fault>>) -> Check {
    match last {
        None => Check::new("Panel", Verdict::Pending, "first frame"),
        Some(Ok(Refreshed::Whole { took })) => Check::new("Panel", Verdict::Pass, format!("whole {} ms", took.as_millis())),
        Some(Ok(Refreshed::Columns { count, took })) => {
            Check::new("Panel", Verdict::Pass, format!("{count} cols {} ms", took.as_millis()))
        }
        Some(Ok(Refreshed::Nothing)) => Check::new("Panel", Verdict::Pass, "unchanged"),
        Some(Err(fault)) => Check::fault("Panel", fault),
    }
}

pub fn clock(reading: &Result<ClockReading, Fault>) -> [Check; 2] {
    match reading {
        Ok(r) => {
            let t = r.time;
            let time = format!("{:02}-{:02} {:02}:{:02}:{:02}", t.month, t.day, t.hour, t.minute, t.second);
            let time = if r.oscillator_stopped {
                Check::new("RTC", Verdict::Fail, format!("unset {time}"))
            } else {
                Check::new("RTC", Verdict::Pass, time)
            };
            [time, Check::new("Alarm", Verdict::Reading, if r.alarm_raised { "raised" } else { "idle" })]
        }
        Err(fault) => [Check::fault("RTC", fault), Check::new("Alarm", Verdict::Fail, "-")],
    }
}

pub fn temperature(reading: &Result<Temperature, Fault>) -> Check {
    match reading {
        Ok(t) => Check::new("Temp", Verdict::Pass, t.to_string()),
        Err(fault) => Check::fault("Temp", fault),
    }
}

pub fn i2c(devices: &[u8], expected: u8) -> Check {
    let list: Vec<String> = devices.iter().map(|a| format!("{a:02x}")).collect();
    let verdict = if devices.contains(&expected) { Verdict::Pass } else { Verdict::Fail };
    Check::new("I2C", verdict, if list.is_empty() { "nothing".into() } else { list.join(" ") })
}

pub fn storage(entries: &Result<Vec<String>, Fault>, capacity_bytes: &Result<u64, Fault>) -> Check {
    match (entries, capacity_bytes) {
        (Err(fault), _) => Check::fault("SD card", fault),
        (Ok(entries), Ok(bytes)) => {
            let tenths_of_gb = bytes * 10 / (1 << 30);
            Check::new("SD card", Verdict::Pass, format!("{} ent {}.{} GB", entries.len(), tenths_of_gb / 10, tenths_of_gb % 10))
        }
        (Ok(entries), Err(_)) => Check::new("SD card", Verdict::Pass, format!("{} entries", entries.len())),
    }
}

pub fn radio(survey: &Survey) -> Check {
    match survey {
        Survey::Scanning => Check::new("Wi-Fi", Verdict::Pending, "scanning..."),
        Survey::Failed(fault) => Check::fault("Wi-Fi", fault),
        Survey::Found(networks) => match networks.iter().map(|n| n.rssi_dbm).max() {
            Some(best) => Check::new("Wi-Fi", Verdict::Pass, format!("{} APs {best} dBm", networks.len())),
            None => Check::new("Wi-Fi", Verdict::Fail, "no network"),
        },
    }
}

pub fn external_power(on: bool) -> Check {
    Check::new("USB", Verdict::Reading, if on { "plugged in" } else { "on battery" })
}

pub fn battery(millivolts: &Result<u32, Fault>) -> Check {
    match millivolts {
        Ok(mv) => Check::new("Battery", Verdict::Reading, format!("{mv} mV at pad")),
        Err(fault) => Check::fault("Battery", fault),
    }
}

pub fn heap(free_bytes: u32) -> Check {
    Check::new("Heap", Verdict::Reading, format!("{} KiB free", free_bytes / 1024))
}

pub fn wheel(detents: i32) -> Check {
    let verdict = if detents == 0 { Verdict::Pending } else { Verdict::Pass };
    Check::new("Wheel", verdict, format!("{detents:+}"))
}

pub fn button(subject: &'static str, presses: u32, held: bool) -> Check {
    let verdict = if presses == 0 { Verdict::Pending } else { Verdict::Pass };
    Check::new(subject, verdict, format!("{presses}{}", if held { " (held)" } else { "" }))
}

pub fn light(subject: &'static str, state: &Result<Brightness, Fault>) -> Check {
    match state {
        Ok(Brightness::OFF) => Check::new(subject, Verdict::Reading, "off"),
        Ok(b) => Check::new(subject, Verdict::Reading, format!("{} %", b.as_percent())),
        Err(fault) => Check::fault(subject, fault),
    }
}

pub fn audio(played: u32, failure: &Option<Fault>) -> Check {
    match failure {
        Some(fault) => Check::fault("Audio", fault),
        None if played == 0 => Check::new("Audio", Verdict::Pending, "silent"),
        None => Check::new("Audio", Verdict::Reading, format!("{played} chime(s)")),
    }
}

pub fn uptime(seconds: u64) -> Check {
    Check::new("Uptime", Verdict::Reading, format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal::clock::DateTime;

    #[test]
    fn a_stopped_oscillator_fails_the_clock() {
        let reading = ClockReading {
            time: DateTime { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 5 },
            oscillator_stopped: true,
            alarm_raised: false,
        };
        let [time, _] = clock(&Ok(reading));
        assert_eq!(time.verdict, Verdict::Fail);
    }

    #[test]
    fn the_i2c_bus_passes_only_with_the_expected_device() {
        assert_eq!(i2c(&[0x68], 0x68).verdict, Verdict::Pass);
        assert_eq!(i2c(&[0x50], 0x68).verdict, Verdict::Fail);
        assert_eq!(i2c(&[], 0x68).reading, "nothing");
    }

    #[test]
    fn storage_capacity_reads_in_gigabytes() {
        let check = storage(&Ok(vec!["a".into(), "b".into()]), &Ok(31_914_983_424));
        assert_eq!(check.reading, "2 ent 29.7 GB");
    }
}
