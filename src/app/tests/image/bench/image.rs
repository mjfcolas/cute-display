use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use app::Devices;
use hal::Fault;
use hal_testing::audio::StubBriefSpeaker;
use hal_testing::display::StubPanel;
use hal_testing::http::StubHttpClient;
use hal_testing::input::{FakeButton, FakeWheel};
use hal_testing::light::FakeLight;
use hal_testing::radio::StubWifiStation;
use hal_testing::running_rtc::FakeRunningRtc;
use hal_testing::stepped_clock::FakeSteppedClock;
use hal_testing::storage::FakeFileStorage;
use hal_testing::system::StubSystemMonitor;
use hal_testing::udp::StubUdpClient;
use ui::gestures::HOLD_TOGETHER;

use super::hardware::TestHardware;
use super::setup::{RtcAtStart, Setup};

/// The main loop, the UI thread and the network thread.
const THREADS_ON_THE_CLOCK: usize = 3;
/// The main loop's period: a press reaches the apps after one turn of the loop.
const MAIN_PERIOD: Duration = Duration::from_millis(100);
/// As long as the stepped clock waits for a slow machine (its `STUCK_AFTER`).
const SPEAKER_ANSWERS_WITHIN: Duration = Duration::from_secs(5);
/// Real time that the speaker's thread needs to take a sound that the image asked for,
/// before a test counts the sounds. The speaker plays on the computer's time, not on the
/// clock's.
const SPEAKER_SETTLES: Duration = Duration::from_millis(50);

/// The running image, and the devices it runs on, which a test acts on and looks at.
pub struct Image {
    pub clock: FakeSteppedClock,
    pub rtc: FakeRunningRtc<FakeSteppedClock>,
    pub card: Option<FakeFileStorage>,
    pub wifi: StubWifiStation,
    pub http: StubHttpClient,
    pub udp: StubUdpClient,
    pub wheel: FakeWheel,
    pub wheel_button: FakeButton,
    pub yellow_button: FakeButton,
    pub long_button: FakeButton,
    pub front_light: FakeLight,
    pub reading_lamp: FakeLight,
    pub speaker: StubBriefSpeaker,
    said: Arc<Mutex<Vec<String>>>,
}

impl Image {
    /// Starts the image and returns once all its threads sleep: its first frame is shown.
    pub fn start(setup: Setup) -> Self {
        let clock = FakeSteppedClock::for_threads(THREADS_ON_THE_CLOCK);
        let rtc = match setup.rtc {
            RtcAtStart::Stopped => FakeRunningRtc::stopped(clock.clone()),
            RtcAtStart::At(time) => FakeRunningRtc::at(time, clock.clone()),
            RtcAtStart::Unreadable => FakeRunningRtc::unreadable("I2C: no answer", clock.clone()),
        };
        let image = Self {
            clock,
            rtc,
            card: setup.card,
            wifi: StubWifiStation::default(),
            http: setup.http,
            udp: setup.udp,
            wheel: FakeWheel::default(),
            wheel_button: FakeButton::default(),
            yellow_button: FakeButton::default(),
            long_button: FakeButton::default(),
            front_light: FakeLight::default(),
            reading_lamp: FakeLight::default(),
            speaker: StubBriefSpeaker::default(),
            said: Arc::default(),
        };
        let devices = Devices::<TestHardware> {
            panel: StubPanel::default(),
            wheel: image.wheel.clone(),
            wheel_button: image.wheel_button.clone(),
            yellow_button: image.yellow_button.clone(),
            long_button: image.long_button.clone(),
            front_light: image.front_light.clone(),
            reading_lamp: image.reading_lamp.clone(),
            rtc: image.rtc.clone(),
            speaker: image.speaker.clone(),
            sd_card: image.card.clone().ok_or_else(|| Fault::new("no card in the slot")),
            wifi: image.wifi.clone(),
            https: image.http.clone(),
            udp: image.udp.clone(),
            system: StubSystemMonitor,
            steady: image.clock.clone(),
        };
        let says = image.said.clone();
        thread::Builder::new()
            .name("main".into())
            .spawn(move || app::run(devices, app::image::OWN_DIRECTORY, catalog::APPS, move |lines| *says.lock().unwrap_or_else(PoisonError::into_inner) = lines))
            .expect("the image's main thread starts");
        image.advance(Duration::ZERO);
        image
    }

    /// Another image on the same card and the same network, with the RTC at this image's
    /// time, or stopped if this one has none; the new RTC always answers. This image is
    /// left asleep, as if it were switched off.
    pub fn restart(&self) -> Self {
        let rtc = self.rtc.time().map_or(RtcAtStart::Stopped, RtcAtStart::At);
        Image::start(Setup { card: self.card.clone(), rtc, udp: self.udp.clone(), http: self.http.clone() })
    }

    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    pub fn said_on_screen(&self) -> Vec<String> {
        self.said.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn screen_line(&self, name: &str) -> Option<String> {
        self.screen_lines(name).into_iter().next()
    }

    pub fn screen_lines(&self, name: &str) -> Vec<String> {
        self.said_on_screen().into_iter().filter(|line| line.split(' ').next() == Some(name)).collect()
    }

    /// On the card, while the image runs.
    pub fn put_file(&self, path: &str, text: &str) {
        self.card.as_ref().expect("a card to put a file on").put(path, text);
    }

    pub fn file(&self, path: &str) -> Option<String> {
        self.card.as_ref()?.file(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn press_long_button(&self) {
        self.long_button.press();
        self.advance(MAIN_PERIOD);
    }

    pub fn press_yellow_button(&self) {
        self.yellow_button.press();
        self.advance(MAIN_PERIOD);
    }

    pub fn click_wheel(&self) {
        self.wheel_button.press();
        self.advance(MAIN_PERIOD);
    }

    pub fn turn_wheel(&self, clockwise_detents: i32) {
        self.wheel.turn(clockwise_detents);
        self.advance(MAIN_PERIOD);
    }

    /// For as long as the gesture needs, plus one more turn of the main loop.
    pub fn hold_yellow_and_long_buttons(&self) {
        for button in [&self.yellow_button, &self.long_button] {
            button.press();
            button.set_held(true);
        }
        self.advance(HOLD_TOGETHER + MAIN_PERIOD);
        for button in [&self.yellow_button, &self.long_button] {
            button.set_held(false);
        }
        self.advance(MAIN_PERIOD);
    }
}

pub fn let_the_speaker_settle() {
    thread::sleep(SPEAKER_SETTLES);
}

/// The speaker's thread plays on the computer's time, not on the clock's.
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
