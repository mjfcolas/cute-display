use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use app::Devices;
use hal::Fault;
use hal_testing::audio::StubBriefSpeaker;
use hal_testing::display::StubPanel;
use hal_testing::input::{FakeButton, FakeWheel};
use hal_testing::light::FakeLight;
use hal_testing::radio::StubWifiStation;
use hal_testing::running_rtc::FakeRunningRtc;
use hal_testing::stepped_clock::FakeSteppedClock;
use hal_testing::system::StubSystemMonitor;
use hal_testing::udp::StubUdpClient;

use super::hardware::TestHardware;
use super::setup::{RtcAtStart, Setup};

/// The main loop, the UI's and the network's.
const THREADS_ON_THE_CLOCK: usize = 3;
/// The main loop's period: what a press does reaches the apps once it has gone round.
const MAIN_PERIOD: Duration = Duration::from_millis(100);
/// As long as the stepped clock gives a slow machine.
const SPEAKER_ANSWERS_WITHIN: Duration = Duration::from_secs(5);

/// The image running, and the devices it runs on, as a test acts on them and looks at them.
pub struct Image {
    pub clock: FakeSteppedClock,
    pub rtc: FakeRunningRtc<FakeSteppedClock>,
    pub udp: StubUdpClient,
    pub long_button: FakeButton,
    pub reading_lamp: FakeLight,
    pub speaker: StubBriefSpeaker,
    said: Arc<Mutex<Vec<String>>>,
}

impl Image {
    /// Started, and left once all its threads are asleep: its first frame is shown.
    pub fn start(setup: Setup) -> Self {
        let clock = FakeSteppedClock::for_threads(THREADS_ON_THE_CLOCK);
        let rtc = match setup.rtc {
            RtcAtStart::Stopped => FakeRunningRtc::stopped(clock.clone()),
            RtcAtStart::At(time) => FakeRunningRtc::at(time, clock.clone()),
        };
        let (long_button, reading_lamp, speaker) = (FakeButton::default(), FakeLight::default(), StubBriefSpeaker::default());
        let devices = Devices::<TestHardware> {
            panel: StubPanel::default(),
            wheel: FakeWheel::default(),
            wheel_button: FakeButton::default(),
            yellow_button: FakeButton::default(),
            long_button: long_button.clone(),
            front_light: FakeLight::default(),
            reading_lamp: reading_lamp.clone(),
            rtc: rtc.clone(),
            speaker: speaker.clone(),
            sd_card: setup.card.ok_or_else(|| Fault::new("no card in the slot")),
            wifi: StubWifiStation,
            https: setup.http,
            udp: setup.udp.clone(),
            system: StubSystemMonitor,
            steady: clock.clone(),
        };
        let said: Arc<Mutex<Vec<String>>> = Arc::default();
        let says = said.clone();
        thread::Builder::new()
            .name("main".into())
            .spawn(move || app::run(devices, catalog::APPS, move |lines| *says.lock().unwrap_or_else(PoisonError::into_inner) = lines))
            .expect("the image's main thread starts");
        let image = Self { clock, rtc, udp: setup.udp, long_button, reading_lamp, speaker, said };
        image.advance(Duration::ZERO);
        image
    }

    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    pub fn said_on_screen(&self) -> Vec<String> {
        self.said.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn screen_line(&self, name: &str) -> Option<String> {
        self.said_on_screen().into_iter().find(|line| line.split(' ').next() == Some(name))
    }

    pub fn press_long_button(&self) {
        self.long_button.press();
        self.advance(MAIN_PERIOD);
    }
}

/// The speaker's thread plays on the computer's time, not the clock's.
pub fn eventually(condition: impl Fn() -> bool) -> bool {
    let until = Instant::now() + SPEAKER_ANSWERS_WITHIN;
    while Instant::now() < until {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}
