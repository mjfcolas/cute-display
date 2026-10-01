use hal::clock::DateTime;
use hal_testing::http::StubHttpClient;
use hal_testing::storage::FakeFileStorage;
use hal_testing::udp::StubUdpClient;

const SATURDAY_AT_SIX: DateTime = DateTime { year: 2026, month: 9, day: 26, hour: 6, minute: 0, second: 0 };

pub const GENERAL_CONF: &str = "cute-display/general.conf";
pub const WIFI_CONF: &str = "cute-display/wifi.conf";
pub const ALARM_CONF: &str = "cute-display/apps/alarm/alarm.conf";

pub enum RtcAtStart {
    Stopped,
    At(DateTime),
}

/// What the device holds before the image starts.
pub struct Setup {
    pub card: Option<FakeFileStorage>,
    pub rtc: RtcAtStart,
    pub udp: StubUdpClient,
    pub http: StubHttpClient,
}

impl Default for Setup {
    /// A card that says the time zone, no Wi-Fi on it, the RTC at six on a Saturday morning.
    fn default() -> Self {
        let card = FakeFileStorage::with(GENERAL_CONF, "place = Paris\nlatitude = 48.85\nlongitude = 2.35\ntime_zone = UTC0\n");
        Self { card: Some(card), rtc: RtcAtStart::At(SATURDAY_AT_SIX), udp: StubUdpClient::silent(), http: StubHttpClient::default() }
    }
}

impl Setup {
    /// On the card, which is put in if there was none.
    pub fn with_file(self, path: &str, text: &str) -> Self {
        let card = self.card.unwrap_or_default();
        card.put(path, text);
        Self { card: Some(card), ..self }
    }
}
