mod controls;
mod engine_services;
pub mod image;
mod network;
mod presentation;
#[cfg(test)]
mod stub_screen;

use std::convert::Infallible;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use domain::apps::{self, AppId, AppService, Foreground};
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
use hal::steady::SteadyClock;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::general_file::{GeneralFile, NoGeneralFile};
use infrastructure::hal_light::HalLight;
use infrastructure::internet::{NoInternet, OnDemandInternet, SharedInternet};
use infrastructure::ntp::NtpServer;
use infrastructure::rtc_keeper::RtcKeeper;
use infrastructure::settings_file::{SettingsFile, Unkept};
use infrastructure::speaker_sound;
use ui::system::OfferedApp;
use ui::Installable;

use crate::controls::Controls;
use crate::engine_services::EngineServices;
use crate::presentation::{AppOnScreen, Presentation};

const MAIN_PERIOD: Duration = Duration::from_millis(100);

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
    type Steady: SteadyClock + Clone + Send + 'static;

    /// The network thread's stack, most of it for what `Http`'s TLS needs.
    const NETWORK_STACK_BYTES: usize;

    /// Starts the speaker's thread, where the samples played may be made as they play, and
    /// read from the card.
    fn spawn_speaker(play: impl FnOnce() + Send + 'static) -> Result<(), Fault>;
}

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
    pub steady: H::Steady,
}

type Service = (AppId, Arc<dyn AppService>);

/// `read_out` hears each frame's lines once the frame is shown.
pub fn run<H: Hardware>(devices: Devices<H>, apps: &[Installable<Frame>], read_out: impl FnMut(Vec<String>) + Send + 'static) -> Result<Infallible, Fault> {
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
    let steady = devices.steady;
    lighting.touched(steady.now());

    let (sound, player) = speaker_sound::sound(devices.speaker);
    H::spawn_speaker(move || player.run())?;

    let keeper = Box::new(RtcKeeper::new(devices.rtc));
    let card = devices.sd_card;
    let internet = card.clone().ok().map(|card| SharedInternet::new(OnDemandInternet::new(devices.wifi, devices.https, devices.udp, card, steady.clone())));
    let clock = match (&card, &internet) {
        (Ok(card), Some(internet)) => {
            Clock::new(keeper, Box::new(NtpServer::new(internet.clone())), Box::new(GeneralFile::new(card.clone())))
        }
        _ => Clock::new(keeper, Box::new(NtpServer::new(NoInternet("no SD card"))), Box::new(NoGeneralFile)),
    };
    clock.tick(steady.now());

    let foreground = Foreground::new(AppId::SYSTEM);
    let engine_services = EngineServices { foreground: foreground.clone(), clock: clock.clone(), card, internet: internet.clone(), sound };
    let wanted = engine_services.card.as_ref().ok().and_then(|card| GeneralFile::new(card.clone()).apps());
    let mut services: Vec<Service> = Vec::new();
    let mut screens: Vec<AppOnScreen> = Vec::new();
    for app in to_install(apps, wanted.as_deref()) {
        let installed = (app.install)(&engine_services.to(app.id));
        services.push((app.id, installed.service));
        screens.push(AppOnScreen { offered: OfferedApp { app: app.id, title: app.title }, screen: installed.screen });
    }
    foreground.bring_to_front(front_at_start(&services));

    network::start(clock.clone(), services.clone(), internet, devices.system, steady.clone(), H::NETWORK_STACK_BYTES)?;

    let controls = Controls {
        wheel: devices.wheel,
        wheel_button: devices.wheel_button,
        yellow_button: devices.yellow_button,
        long_button: devices.long_button,
    };
    let presentation = Presentation { controls, panel: devices.panel, read_out, steady: steady.clone(), started: steady.now() };
    let shown_lighting = lighting.clone();
    thread::Builder::new()
        .name("ui".into())
        .stack_size(32 * 1024)
        .spawn(move || presentation.run(foreground, settings, shown_lighting, screens))
        .map_err(Fault::new)?;

    loop {
        let now = steady.now();
        clock.tick(now);
        let local = clock.now();
        for (_, service) in &services {
            service.tick(now, local);
        }
        lighting.shine_at_least(light_wanted(&services));
        lighting.refresh(now);
        steady.sleep(MAIN_PERIOD);
    }
}

fn to_install<'a>(apps: &'a [Installable<Frame>], wanted: Option<&[String]>) -> impl Iterator<Item = &'a Installable<Frame>> {
    let image: Vec<AppId> = apps.iter().map(|app| app.id).collect();
    for name in wanted.unwrap_or_default() {
        if !image.iter().any(|app| app.name() == name) {
            log::warn!("apps: this image has no {name:?}");
        }
    }
    apps::chosen(&image, wanted).into_iter().filter_map(move |id| apps.iter().find(|app| app.id == id))
}

fn front_at_start(services: &[Service]) -> AppId {
    services.first().map_or(AppId::SYSTEM, |(app, _)| *app)
}

fn light_wanted(services: &[Service]) -> Level {
    services.iter().map(|(_, service)| service.light()).max().unwrap_or(Level::OFF)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::stub_screen::StubScreen;

    struct StubAppService(Level);

    impl AppService for StubAppService {
        fn light(&self) -> Level {
            self.0
        }
    }

    fn install(_: &dyn domain::apps::Services) -> ui::InstalledApp<Frame> {
        ui::InstalledApp { service: Arc::new(StubAppService(Level::OFF)), screen: Box::new(StubScreen) }
    }

    fn installable(name: &'static str) -> Installable<Frame> {
        Installable { id: AppId::new(name), title: name, install }
    }

    #[test]
    fn the_apps_installed_are_those_chosen_in_their_order_once_each() {
        let image = [installable("alarm"), installable("weather"), installable("radar")];
        let names = |wanted: Option<&[String]>| -> Vec<&str> { to_install(&image, wanted).map(|app| app.id.name()).collect() };
        assert_eq!(names(None), ["alarm", "weather", "radar"]);
        let wanted = ["radar", "clock", "alarm", "radar"].map(String::from);
        assert_eq!(names(Some(&wanted)), ["radar", "alarm"]);
    }

    fn service(name: &'static str, light: u8) -> Service {
        (AppId::new(name), Arc::new(StubAppService(Level::percent(light))))
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
