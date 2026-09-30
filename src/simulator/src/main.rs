mod card;
mod clock;
mod console;
mod controls;
mod lights;
mod network;
mod options;
mod panel;
mod speaker;
mod steady;
mod system;
mod window;

use std::thread;

use app::{Devices, Hardware};
use drivers::udp_socket::StdUdpClient;
use hal::storage::FileStorage;
use hal::Fault;
use infrastructure::composite_input::{CompositeButton, CompositeWheel};
use infrastructure::internet::WIFI_FILE;
use maintenance::remote::{ButtonName, Remote, RemoteButton, RemoteWheel};
use maintenance::MaintenanceConsole;

use crate::card::DirectoryCard;
use crate::clock::HostClock;
use crate::controls::{KeyButton, ScrollWheel};
use crate::lights::SimulatedLight;
use crate::network::{HostHttpClient, HostWifi};
use crate::options::Options;
use crate::panel::SimulatedPanel;
use crate::speaker::LoggedSpeaker;
use crate::steady::ScaledClock;
use crate::system::HostSystem;
use crate::window::Case;

enum Computer {}

impl Hardware for Computer {
    type Panel = SimulatedPanel;
    type Wheel = CompositeWheel<ScrollWheel, RemoteWheel>;
    type Button = CompositeButton<KeyButton, RemoteButton>;
    type Light = SimulatedLight;
    type Rtc = HostClock;
    type Speaker = LoggedSpeaker;
    type Card = DirectoryCard;
    type Wifi = HostWifi;
    type Http = HostHttpClient;
    type Udp = StdUdpClient;
    type System = HostSystem;
    type Steady = ScaledClock;

    /// Rust's own default: rustls, unoptimised, needs far more than the ESP32's TLS.
    const NETWORK_STACK_BYTES: usize = 2 * 1024 * 1024;

    fn spawn_speaker(play: impl FnOnce() + Send + 'static) -> Result<(), Fault> {
        thread::Builder::new().name("speaker".into()).spawn(play).map(drop).map_err(Fault::new)
    }
}

/// The window has the main thread: some systems deliver its events on no other.
fn main() -> Result<(), Fault> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let options = Options::parse(std::env::args().skip(1)).map_err(Fault::new)?;
    let card = DirectoryCard::open(&options.card).and_then(with_wifi);
    match &card {
        Ok(_) => log::info!("SD card: {}", options.card.display()),
        Err(fault) => log::warn!("no SD card: {fault}"),
    }

    let steady = ScaledClock::new(options.speed);
    let case = Case::new(steady);
    let remote = Remote::new(steady);
    if let Some(path) = &options.console_socket {
        console::listen(path, MaintenanceConsole::new(card.clone(), remote.clone()))?;
    }
    let devices = Devices::<Computer> {
        panel: case.panel.clone(),
        wheel: CompositeWheel::new(case.wheel.clone(), remote.wheel()),
        wheel_button: CompositeButton::new(case.wheel_button.clone(), remote.button(ButtonName::WheelButton)),
        yellow_button: CompositeButton::new(case.yellow_button.clone(), remote.button(ButtonName::Yellow)),
        long_button: CompositeButton::new(case.long_button.clone(), remote.button(ButtonName::Long)),
        front_light: case.front_light.clone(),
        reading_lamp: case.reading_lamp.clone(),
        rtc: HostClock::new(steady)?,
        speaker: LoggedSpeaker(steady),
        sd_card: card,
        wifi: HostWifi,
        https: HostHttpClient::default(),
        udp: StdUdpClient,
        system: HostSystem,
        steady,
    };
    thread::Builder::new()
        .name("app".into())
        .spawn(move || match app::run(devices, catalog::APPS) {
            Ok(never) => match never {},
            Err(fault) => log::error!("app: {fault}"),
        })
        .map_err(Fault::new)?;

    window::show(&case)
}

/// The Internet asks the card for a network to join; the computer is already on one.
fn with_wifi(card: DirectoryCard) -> Result<DirectoryCard, Fault> {
    if card.read(WIFI_FILE)?.is_none() {
        card.write(WIFI_FILE, b"ssid = the computer's own connection\n")?;
    }
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_gets_a_wifi_conf_only_when_it_has_none() {
        let root = std::env::temp_dir().join(format!("cute-display-wifi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let card = with_wifi(DirectoryCard::open(&root).unwrap()).unwrap();
        assert!(card.read(WIFI_FILE).unwrap().is_some());

        card.write(WIFI_FILE, b"ssid = Home\n").unwrap();
        let card = with_wifi(card).unwrap();
        assert_eq!(card.read(WIFI_FILE).unwrap().as_deref(), Some(&b"ssid = Home\n"[..]));
    }
}
