//! The app image, on any hardware that keeps the HAL's contracts: which threads run and
//! how the layers are wired. `firmware` runs it on the board, `simulator` on a computer.

mod controls;
mod engine_services;
pub mod image;
mod network;
mod presentation;

use std::convert::Infallible;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use domain::apps::{AppId, AppService, Foreground};
use domain::clock::Clock;
use domain::lighting::{Level, Lighting};
use domain::settings::{Settings, SettingsStore};
use hal::audio::Speaker;
use hal::clock::RealTimeClock;
use hal::display::{EpaperDisplay, Frame};
use hal::http::HttpClient;
use hal::input::{PushButton, RotaryEncoder};
use hal::light::DimmableLight;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::hal_light::HalLight;
use infrastructure::internet::{NoInternet, OnDemandInternet, SharedInternet};
use infrastructure::ntp::NtpServer;
use infrastructure::rtc_keeper::RtcKeeper;
use infrastructure::settings_file::{SettingsFile, Unkept};
use infrastructure::speaker_sound::{self, SoundPlayer};
use infrastructure::time_zone_file::{NoTimeZone, TimeZoneFile};
use ui::{AppScreen, Install};

use crate::controls::Controls;
use crate::engine_services::EngineServices;
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

/// An installed app's service, and the app it is.
type Service = (AppId, Arc<dyn AppService>);

/// Installs `apps`, the first of them in front, then starts the ui, speaker and network
/// threads, and keeps the time, the apps' services and the lights on the calling thread.
/// Returns only when a thread could not be started.
pub fn run<H: Hardware>(devices: Devices<H>, apps: &[Install<Frame>]) -> Result<Infallible, Fault> {
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

    let (sound, player) = speaker_sound::sound(devices.speaker);
    start_speaker(player)?;

    let keeper = Box::new(RtcKeeper::new(devices.rtc));
    let card = devices.sd_card;
    let internet = card.clone().ok().map(|card| SharedInternet::new(OnDemandInternet::new(devices.wifi, devices.https, devices.udp, card)));
    let clock = match (&card, &internet) {
        (Ok(card), Some(internet)) => {
            Clock::new(keeper, Box::new(NtpServer::new(internet.clone())), Box::new(TimeZoneFile::new(card.clone())))
        }
        _ => Clock::new(keeper, Box::new(NtpServer::new(NoInternet("no SD card"))), Box::new(NoTimeZone)),
    };
    clock.tick(Instant::now());

    let foreground = Foreground::new(AppId::SYSTEM);
    let engine_services = EngineServices { foreground: foreground.clone(), clock: clock.clone(), card, internet: internet.clone(), sound };
    let installed: Vec<_> = apps.iter().map(|install| install(&engine_services)).collect();
    let services: Vec<Service> = installed.iter().map(|app| (app.screen.app(), Arc::clone(&app.service))).collect();
    foreground.bring_to_front(front_at_start(&services));
    let screens: Vec<Box<dyn AppScreen<Frame> + Send>> = installed.into_iter().map(|app| app.screen).collect();

    network::start(clock.clone(), services.clone(), internet, devices.system, H::NETWORK_STACK_BYTES)?;

    let controls = Controls {
        wheel: devices.wheel,
        wheel_button: devices.wheel_button,
        yellow_button: devices.yellow_button,
        long_button: devices.long_button,
    };
    let presentation = Presentation { controls, panel: devices.panel };
    let shown_lighting = lighting.clone();
    thread::Builder::new()
        .name("ui".into())
        .stack_size(32 * 1024)
        .spawn(move || presentation.run(foreground, settings, shown_lighting, screens))
        .map_err(Fault::new)?;

    loop {
        let now = Instant::now();
        clock.tick(now);
        let local = clock.now();
        for (_, service) in &services {
            service.tick(now, local);
        }
        lighting.shine_at_least(light_wanted(&services));
        lighting.refresh(now);
        thread::sleep(MAIN_PERIOD);
    }
}

/// The first app installed, or the system app when there is none.
fn front_at_start(services: &[Service]) -> AppId {
    services.first().map_or(AppId::SYSTEM, |(app, _)| *app)
}

/// The lights follow the app that wants the most.
fn light_wanted(services: &[Service]) -> Level {
    services.iter().map(|(_, service)| service.light()).max().unwrap_or(Level::OFF)
}

fn start_speaker<S: Speaker + Send + 'static>(player: SoundPlayer<S>) -> Result<(), Fault> {
    thread::Builder::new().name("speaker".into()).stack_size(8 * 1024).spawn(move || player.run()).map_err(Fault::new)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lamp(Level);

    impl AppService for Lamp {
        fn light(&self) -> Level {
            self.0
        }
    }

    fn service(name: &'static str, light: u8) -> Service {
        (AppId::new(name), Arc::new(Lamp(Level::percent(light))))
    }

    #[test]
    fn the_first_app_installed_is_in_front_and_without_any_the_system_app() {
        assert_eq!(front_at_start(&[service("alarm", 0), service("radar", 0)]), AppId::new("alarm"));
        assert_eq!(front_at_start(&[]), AppId::SYSTEM);
    }

    #[test]
    fn the_lights_follow_the_app_that_wants_the_most() {
        assert_eq!(light_wanted(&[service("alarm", 40), service("radar", 0), service("lamp", 70)]), Level::percent(70));
        assert_eq!(light_wanted(&[]), Level::OFF);
    }
}
