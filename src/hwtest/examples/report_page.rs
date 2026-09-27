//! Writes the report page, filled with plausible checks, as a raw frame:
//!
//!   cargo run -p hwtest --example report_page -- out.fb [pattern]

use hwtest::report::{Check, Report, Verdict};
use hwtest::screen::{self, Legend, Page};

fn check(subject: &'static str, verdict: Verdict, reading: &str) -> Check {
    Check { subject, verdict, reading: reading.into() }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).map_or("/tmp/cute-display.fb", String::as_str);
    let page = if args.get(2).is_some_and(|a| a == "pattern") {
        Page::TestPattern
    } else {
        Page::Report(Report {
            hardware: vec![
                check("Panel", Verdict::Pass, "187 cols 350 ms"),
                check("RTC", Verdict::Pass, "09-25 17:42:07"),
                check("Alarm", Verdict::Reading, "idle"),
                check("Temp", Verdict::Pass, "24.75 C"),
                check("I2C", Verdict::Pass, "68"),
                check("SD card", Verdict::Pass, "7 ent 29.7 GB"),
                check("Wi-Fi", Verdict::Pass, "11 APs -48 dBm"),
                check("USB", Verdict::Reading, "plugged in"),
                check("Battery", Verdict::Reading, "907 mV at pad"),
                check("Heap", Verdict::Reading, "212 KiB free"),
            ],
            controls: vec![
                check("Wheel", Verdict::Pass, "+12"),
                check("Press", Verdict::Pass, "3"),
                check("Yellow", Verdict::Pending, "0"),
                check("Long", Verdict::Fail, "stuck"),
                check("Front", Verdict::Reading, "40 %"),
                check("Lamp", Verdict::Reading, "off"),
                check("Audio", Verdict::Reading, "4 chime(s)"),
                check("Uptime", Verdict::Reading, "0:03:27"),
            ],
        })
    };
    let legend = Legend {
        title: "Cute Display  hardware test",
        help: "wheel: front light  press: chime  yellow: lamp  long: pattern",
        build: "build 09-25 17:40Z @preview",
    };
    std::fs::write(out, screen::draw(&page, &legend).as_bytes())
}
