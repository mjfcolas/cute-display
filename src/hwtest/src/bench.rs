use std::thread;
use std::time::{Duration, Instant};

use hal::audio::Speaker;
use hal::clock::{ClockReading, RealTimeClock};
use hal::display::EpaperDisplay;
use hal::input::{PushButton, RotaryEncoder};
use hal::light::{Brightness, DimmableLight};
use hal::power::PowerMonitor;
use hal::radio::WifiScanner;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::thermometer::{Temperature, Thermometer};
use hal::Fault;

use crate::chime::Chimes;
use crate::latest::Latest;
use crate::painter::Painter;
use crate::report::{self, Check, Report};
use crate::screen::{Legend, Page};
use crate::survey::{self, Survey};

const STEP_PERIOD: Duration = Duration::from_millis(20);
const SENSE_PERIOD: Duration = Duration::from_secs(1);
const FRONT_LIGHT_PER_DETENT: i32 = 10;
const FRONT_LIGHT_AT_START: Brightness = Brightness::percent(20);
const READING_LAMP_STEPS: [u8; 4] = [0, 10, 50, 100];
const RTC_ADDRESS: u8 = 0x68;

const TITLE: &str = "Cute Display  hardware test";
const HELP: &str = "wheel: front light  press: chime  yellow: lamp  long: pattern";

pub struct Controls {
    pub wheel: Box<dyn RotaryEncoder>,
    pub wheel_button: Box<dyn PushButton>,
    pub yellow_button: Box<dyn PushButton>,
    pub long_button: Box<dyn PushButton>,
}

pub struct Lights {
    pub front: Box<dyn DimmableLight>,
    pub reading_lamp: Box<dyn DimmableLight>,
}

pub struct Sensors {
    pub clock: Box<dyn RealTimeClock>,
    pub thermometer: Box<dyn Thermometer>,
    pub i2c_devices: Vec<u8>,
    pub storage: Result<Box<dyn FileStorage>, Fault>,
    pub power: Box<dyn PowerMonitor>,
    pub system: Box<dyn SystemMonitor>,
}

/// Devices that block for long enough to get a thread each.
pub struct Blocking<D, S, W> {
    pub display: D,
    pub speaker: S,
    pub wifi: W,
}

#[derive(Default)]
struct Tally {
    detents: i32,
    wheel_presses: u32,
    yellow_presses: u32,
    long_presses: u32,
    reading_lamp_step: usize,
}

struct Readings {
    clock: Result<ClockReading, Fault>,
    temperature: Result<Temperature, Fault>,
    on_external_power: bool,
    battery: Result<u32, Fault>,
    free_heap: u32,
}

pub struct Bench {
    controls: Controls,
    lights: Lights,
    sensors: Sensors,
    painter: Painter,
    chimes: Chimes,
    survey: Latest<Survey>,
    storage: Check,
    front_light: Result<Brightness, Fault>,
    reading_lamp: Result<Brightness, Fault>,
    tally: Tally,
    readings: Option<(Instant, Readings)>,
    showing_pattern: bool,
    started: Instant,
}

impl Bench {
    pub fn start<D, S, W>(
        controls: Controls,
        mut lights: Lights,
        sensors: Sensors,
        blocking: Blocking<D, S, W>,
        build: &'static str,
    ) -> Result<Self, Fault>
    where
        D: EpaperDisplay + Send + 'static,
        S: Speaker + Send + 'static,
        W: WifiScanner + Send + 'static,
    {
        let front_light = set(lights.front.as_mut(), FRONT_LIGHT_AT_START);
        let reading_lamp = set(lights.reading_lamp.as_mut(), Brightness::OFF);
        let storage = match &sensors.storage {
            Ok(card) => report::storage(&card.entries("").map(Option::unwrap_or_default), &card.capacity_bytes()),
            Err(fault) => report::storage(&Err(fault.clone()), &Err(fault.clone())),
        };
        let chimes = Chimes::spawn(blocking.speaker)?;
        chimes.ring();
        Ok(Self {
            controls,
            lights,
            sensors,
            painter: Painter::spawn(blocking.display, Legend { title: TITLE, help: HELP, build })?,
            chimes,
            survey: survey::spawn(blocking.wifi)?,
            storage,
            front_light,
            reading_lamp,
            tally: Tally::default(),
            readings: None,
            showing_pattern: false,
            started: Instant::now(),
        })
    }

    pub fn run(mut self) -> ! {
        loop {
            self.step();
            thread::sleep(STEP_PERIOD);
        }
    }

    pub fn step(&mut self) {
        let touched = self.follow_controls();
        let sense_due = self.readings.as_ref().is_none_or(|(at, _)| at.elapsed() >= SENSE_PERIOD);
        if sense_due {
            self.readings = Some((Instant::now(), self.sense()));
        }
        let report_changed = !self.showing_pattern && (sense_due || touched.anything);
        if touched.page_toggled || report_changed {
            self.painter.paint(self.page());
        }
    }

    fn follow_controls(&mut self) -> Touched {
        let detents = self.controls.wheel.take_detents();
        let wheel_presses = self.controls.wheel_button.take_presses();
        let yellow_presses = self.controls.yellow_button.take_presses();
        let long_presses = self.controls.long_button.take_presses();

        if detents != 0 {
            self.tally.detents += detents;
            let wanted = self.lights.front.brightness().adjusted_by(detents * FRONT_LIGHT_PER_DETENT);
            self.front_light = set(self.lights.front.as_mut(), wanted);
            log::info!("wheel {detents:+}: front light {} %", wanted.as_percent());
        }
        if wheel_presses > 0 {
            self.tally.wheel_presses += wheel_presses;
            self.chimes.ring();
            log::info!("wheel press: chime");
        }
        if yellow_presses > 0 {
            self.tally.yellow_presses += yellow_presses;
            self.tally.reading_lamp_step = (self.tally.reading_lamp_step + yellow_presses as usize) % READING_LAMP_STEPS.len();
            let percent = READING_LAMP_STEPS.get(self.tally.reading_lamp_step).copied().unwrap_or(0);
            self.reading_lamp = set(self.lights.reading_lamp.as_mut(), Brightness::percent(percent));
            log::info!("yellow: reading lamp {percent} %");
        }
        let page_toggled = long_presses % 2 == 1;
        if long_presses > 0 {
            self.tally.long_presses += long_presses;
            self.showing_pattern ^= page_toggled;
            log::info!("long: {}", if self.showing_pattern { "test pattern" } else { "report" });
        }

        Touched {
            anything: detents != 0 || wheel_presses + yellow_presses + long_presses > 0,
            page_toggled,
        }
    }

    fn sense(&mut self) -> Readings {
        Readings {
            clock: self.sensors.clock.read(),
            temperature: self.sensors.thermometer.temperature(),
            on_external_power: self.sensors.power.on_external_power(),
            battery: self.sensors.power.battery_sense_millivolts(),
            free_heap: self.sensors.system.free_heap_bytes(),
        }
    }

    fn page(&self) -> Page {
        if self.showing_pattern {
            return Page::TestPattern;
        }
        Page::Report(self.report())
    }

    fn report(&self) -> Report {
        let mut hardware = vec![report::panel(&self.painter.last_refresh())];
        if let Some((_, r)) = &self.readings {
            hardware.extend(report::clock(&r.clock));
            hardware.push(report::temperature(&r.temperature));
        }
        hardware.push(report::i2c(&self.sensors.i2c_devices, RTC_ADDRESS));
        hardware.push(self.storage.clone());
        hardware.push(report::radio(&self.survey.get()));
        if let Some((_, r)) = &self.readings {
            hardware.push(report::external_power(r.on_external_power));
            hardware.push(report::battery(&r.battery));
            hardware.push(report::heap(r.free_heap));
        }

        let buttons = &self.controls;
        let controls = vec![
            report::wheel(self.tally.detents),
            report::button("Press", self.tally.wheel_presses, buttons.wheel_button.is_held()),
            report::button("Yellow", self.tally.yellow_presses, buttons.yellow_button.is_held()),
            report::button("Long", self.tally.long_presses, buttons.long_button.is_held()),
            report::light("Front", &self.front_light),
            report::light("Lamp", &self.reading_lamp),
            report::audio(self.chimes.played(), &self.chimes.failure()),
            report::uptime(self.started.elapsed().as_secs()),
        ];
        Report { hardware, controls }
    }
}

struct Touched {
    anything: bool,
    page_toggled: bool,
}

fn set(light: &mut dyn DimmableLight, brightness: Brightness) -> Result<Brightness, Fault> {
    light.set_brightness(brightness).map(|()| light.brightness())
}

#[cfg(test)]
mod tests {
    use hal::display::Redraw;
    use hal::radio::AccessPoint;
    use hal_testing::audio::StubSpeaker;
    use hal_testing::clock::FakeRtc;
    use hal_testing::display::StubPanel;
    use hal_testing::input::{FakeButton, FakeWheel};
    use hal_testing::light::FakeLight;
    use hal_testing::system::StubSystemMonitor;

    use super::*;

    struct StubThermometer;
    impl Thermometer for StubThermometer {
        fn temperature(&mut self) -> Result<Temperature, Fault> {
            Ok(Temperature { quarter_degrees_celsius: 90 })
        }
    }

    struct StubPowerMonitor;
    impl PowerMonitor for StubPowerMonitor {
        fn on_external_power(&self) -> bool {
            true
        }
        fn battery_sense_millivolts(&mut self) -> Result<u32, Fault> {
            Ok(900)
        }
    }

    struct StubWifiScanner;
    impl WifiScanner for StubWifiScanner {
        fn scan(&mut self) -> Result<Vec<AccessPoint>, Fault> {
            Ok(vec![])
        }
    }

    #[derive(Default)]
    struct Hands {
        wheel: FakeWheel,
        wheel_button: FakeButton,
        yellow_button: FakeButton,
        long_button: FakeButton,
        front: FakeLight,
        lamp: FakeLight,
        panel: StubPanel,
        speaker: StubSpeaker,
    }

    #[derive(Debug, PartialEq)]
    enum Seen {
        Report,
        Pattern,
    }

    impl Hands {
        fn last_shown(&self) -> Option<(Redraw, Seen)> {
            // The pattern's top-left square is ink, where the report leaves a margin.
            self.panel.last_shown().map(|(redraw, frame)| (redraw, if frame.is_ink(5, 5) { Seen::Pattern } else { Seen::Report }))
        }
    }

    fn bench() -> (Bench, Hands) {
        let hands = Hands::default();
        let controls = Controls {
            wheel: Box::new(hands.wheel.clone()),
            wheel_button: Box::new(hands.wheel_button.clone()),
            yellow_button: Box::new(hands.yellow_button.clone()),
            long_button: Box::new(hands.long_button.clone()),
        };
        let lights = Lights { front: Box::new(hands.front.clone()), reading_lamp: Box::new(hands.lamp.clone()) };
        let sensors = Sensors {
            clock: Box::new(FakeRtc::unreadable("absent")),
            thermometer: Box::new(StubThermometer),
            i2c_devices: vec![RTC_ADDRESS],
            storage: Err(Fault::new("no card")),
            power: Box::new(StubPowerMonitor),
            system: Box::new(StubSystemMonitor),
        };
        let blocking = Blocking { display: hands.panel.clone(), speaker: hands.speaker.clone(), wifi: StubWifiScanner };
        (Bench::start(controls, lights, sensors, blocking, "test").unwrap(), hands)
    }

    fn eventually(condition: impl Fn() -> bool) -> bool {
        (0..200).any(|_| {
            thread::sleep(Duration::from_millis(5));
            condition()
        })
    }

    #[test]
    fn the_wheel_steers_the_front_light() {
        let (mut bench, hands) = bench();
        assert_eq!(hands.front.brightness(), FRONT_LIGHT_AT_START);
        hands.wheel.turn(3);
        bench.step();
        assert_eq!(hands.front.brightness().as_percent(), 50);
        hands.wheel.turn(-9);
        bench.step();
        assert_eq!(hands.front.brightness(), Brightness::OFF);
    }

    #[test]
    fn the_yellow_button_cycles_the_reading_lamp() {
        let (mut bench, hands) = bench();
        (0..2).for_each(|_| hands.yellow_button.press());
        bench.step();
        assert_eq!(hands.lamp.brightness().as_percent(), 50);
        (0..3).for_each(|_| hands.yellow_button.press());
        bench.step();
        assert_eq!(hands.lamp.brightness().as_percent(), 10);
    }

    #[test]
    fn a_wheel_press_rings_a_chime_after_the_one_at_start() {
        let (mut bench, hands) = bench();
        hands.wheel_button.press();
        bench.step();
        assert!(eventually(|| hands.speaker.sounds_played() == 2));
    }

    #[test]
    fn the_long_button_swaps_the_report_for_the_pattern_with_a_whole_redraw() {
        let (mut bench, hands) = bench();
        bench.step();
        assert!(eventually(|| hands.panel.times_shown() > 0));
        hands.long_button.press();
        bench.step();
        assert!(eventually(|| hands.last_shown() == Some((Redraw::Whole, Seen::Pattern))));
        hands.long_button.press();
        bench.step();
        assert!(eventually(|| hands.last_shown() == Some((Redraw::Whole, Seen::Report))));
    }
}
