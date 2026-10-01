use std::thread;

use app::Hardware;
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

pub enum TestHardware {}

impl Hardware for TestHardware {
    type Panel = StubPanel;
    type Wheel = FakeWheel;
    type Button = FakeButton;
    type Light = FakeLight;
    type Rtc = FakeRunningRtc<FakeSteppedClock>;
    type Speaker = StubBriefSpeaker;
    type Card = FakeFileStorage;
    type Wifi = StubWifiStation;
    type Http = StubHttpClient;
    type Udp = StubUdpClient;
    type System = StubSystemMonitor;
    type Steady = FakeSteppedClock;

    const NETWORK_STACK_BYTES: usize = 2 * 1024 * 1024;

    fn spawn_speaker(play: impl FnOnce() + Send + 'static) -> Result<(), Fault> {
        thread::Builder::new().name("speaker".into()).spawn(play).map(drop).map_err(Fault::new)
    }
}
