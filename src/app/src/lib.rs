//! The app image, on any hardware that keeps the HAL's contracts: which threads run and
//! how the layers are wired. `firmware` runs it on the board, `simulator` on a computer.

mod controls;
pub mod image;
mod network;
mod presentation;

use std::convert::Infallible;
use std::thread;
use std::time::{Duration, Instant};

use domain::alarm::{AlarmClock, AlarmScheduleStore};
use domain::apps::{App, Foreground};
use domain::clock::Clock;
use domain::lighting::Lighting;
use domain::radar::Radar;
use domain::settings::{Settings, SettingsStore};
use domain::weather::Weather;
use hal::audio::Speaker;
use hal::clock::RealTimeClock;
use hal::display::EpaperDisplay;
use hal::http::HttpClient;
use hal::input::{PushButton, RotaryEncoder};
use hal::light::DimmableLight;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::adsb_fi::AdsbFi;
use infrastructure::alarm_file::{AlarmFile, UnkeptAlarm};
use infrastructure::airports_file::{AirportsFile, NoAirports};
use infrastructure::hal_light::HalLight;
use infrastructure::internet::{NoInternet, OnDemandInternet, SharedInternet};
use infrastructure::ntp::NtpServer;
use infrastructure::open_meteo::OpenMeteo;
use infrastructure::place_file::{NoPlace, PlaceFile, RADAR_FILE, WEATHER_FILE};
use infrastructure::rtc_keeper::RtcKeeper;
use infrastructure::settings_file::{SettingsFile, Unkept};
use infrastructure::speaker_ringer::{self, RingtonePlayer};
use infrastructure::time_zone_file::{NoTimeZone, TimeZoneFile};

use crate::controls::Controls;
use crate::presentation::Presentation;

const MAIN_PERIOD: Duration = Duration::from_millis(100);

/// The devices of one kind of hardware.
pub trait Hardware {
    type Panel: EpaperDisplay + Send + 'static;
    type Wheel: RotaryEncoder + Send + 'static;
    type Button: PushButton + Send + 'static;
    type Light: DimmableLight + Send + 'static;
    type Rtc: RealTimeClock + Send + 'static;
    type Speaker: Speaker + Send + 'static;
    type Card: FileStorage + Clone + Send + 'static;
    type Wifi: WifiStation + Send + 'static;
    type Http: HttpClient + Send + 'static;
    type Udp: UdpClient + Send + 'static;
    type System: SystemMonitor + Send + 'static;

    /// The network thread's stack, most of it for what `Http`'s TLS needs.
    const NETWORK_STACK_BYTES: usize;
}

/// What the app image uses of the hardware.
pub struct Devices<H: Hardware> {
    pub panel: H::Panel,
    pub wheel: H::Wheel,
    pub wheel_button: H::Button,
    pub yellow_button: H::Button,
    pub long_button: H::Button,
    pub front_light: H::Light,
    pub reading_lamp: H::Light,
    pub rtc: H::Rtc,
    pub speaker: H::Speaker,
    pub sd_card: Result<H::Card, Fault>,
    pub wifi: H::Wifi,
    pub https: H::Http,
    pub udp: H::Udp,
    pub system: H::System,
}

/// Everything the domain is made of, shared by the threads that use it.
struct Domain {
    foreground: Foreground,
    clock: Clock,
    alarm: AlarmClock,
    settings: Settings,
    lighting: Lighting,
    weather: Weather,
    radar: Radar,
}

/// Starts the ui, speaker and network threads, then keeps the time, the alarm and the
/// lights on the calling thread. Returns only when a thread could not be started.
pub fn run<H: Hardware>(devices: Devices<H>) -> Result<Infallible, Fault> {
    let settings_store: Box<dyn SettingsStore> = match &devices.sd_card {
        Ok(card) => Box::new(SettingsFile::new(card.clone())),
        Err(fault) => {
            log::warn!("settings: no SD card ({fault}); they will not survive a power cut");
            Box::new(Unkept)
        }
    };
    let settings = Settings::load(settings_store);
    let lighting = Lighting::new(
        Box::new(HalLight::new(devices.front_light)),
        Box::new(HalLight::new(devices.reading_lamp)),
        settings.clone(),
    );
    lighting.touched(Instant::now());

    let foreground = Foreground::new(App::Alarm);
    let alarm_store: Box<dyn AlarmScheduleStore> = match &devices.sd_card {
        Ok(card) => Box::new(AlarmFile::new(card.clone())),
        Err(_) => Box::new(UnkeptAlarm),
    };
    let (ringer, player) = speaker_ringer::ringer(devices.speaker);
    start_speaker(player)?;
    let alarm = AlarmClock::new(alarm_store, Box::new(ringer), foreground.clone());

    let keeper = Box::new(RtcKeeper::new(devices.rtc));
    let (clock, weather, radar, internet) = match devices.sd_card {
        Ok(card) => {
            let internet = SharedInternet::new(OnDemandInternet::new(devices.wifi, devices.https, devices.udp, card.clone()));
            let clock = Clock::new(keeper, Box::new(NtpServer::new(internet.clone())), Box::new(TimeZoneFile::new(card.clone())));
            let weather = Weather::new(
                Box::new(PlaceFile::new(card.clone(), WEATHER_FILE)),
                Box::new(OpenMeteo::new(internet.clone())),
            );
            let radar = Radar::new(
                Box::new(PlaceFile::new(card.clone(), RADAR_FILE)),
                Box::new(AdsbFi::new(internet.clone())),
                Box::new(AirportsFile::new(card)),
                foreground.clone(),
            );
            (clock, weather, radar, Some(internet))
        }
        Err(_) => (
            Clock::new(keeper, Box::new(NtpServer::new(NoInternet("no SD card"))), Box::new(NoTimeZone)),
            Weather::new(Box::new(NoPlace), Box::new(OpenMeteo::new(NoInternet("no SD card")))),
            Radar::new(
                Box::new(NoPlace),
                Box::new(AdsbFi::new(NoInternet("no SD card"))),
                Box::new(NoAirports),
                foreground.clone(),
            ),
            None,
        ),
    };
    clock.tick(Instant::now());
    network::start(clock.clone(), weather.clone(), radar.clone(), internet, devices.system, H::NETWORK_STACK_BYTES)?;

    let domain = Domain { foreground, clock: clock.clone(), alarm: alarm.clone(), settings, lighting: lighting.clone(), weather, radar };
    let controls = Controls {
        wheel: devices.wheel,
        wheel_button: devices.wheel_button,
        yellow_button: devices.yellow_button,
        long_button: devices.long_button,
    };
    let presentation = Presentation { controls, panel: devices.panel };
    thread::Builder::new()
        .name("ui".into())
        .stack_size(32 * 1024)
        .spawn(move || presentation.run(domain))
        .map_err(Fault::new)?;

    loop {
        let now = Instant::now();
        clock.tick(now);
        if let Some(local) = clock.now() {
            alarm.tick(local);
        }
        lighting.rise_sun_to(alarm.sunrise());
        lighting.refresh(now);
        thread::sleep(MAIN_PERIOD);
    }
}

fn start_speaker<S: Speaker + Send + 'static>(player: RingtonePlayer<S>) -> Result<(), Fault> {
    thread::Builder::new().name("speaker".into()).stack_size(8 * 1024).spawn(move || player.run()).map_err(Fault::new)?;
    Ok(())
}
