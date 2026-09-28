//! The app image on a computer: `just sim [card directory]`.

mod card;
mod clock;
mod controls;
mod lights;
mod network;
mod panel;
mod speaker;
mod system;
mod window;

use std::path::PathBuf;
use std::thread;

use app::{Devices, Hardware};
use drivers::udp_socket::StdUdpClient;
use hal::display::Frame;
use hal::storage::FileStorage;
use hal::Fault;
use infrastructure::internet::WIFI_FILE;
use ui::Install;

use crate::card::DirectoryCard;
use crate::clock::HostClock;
use crate::controls::{KeyButton, ScrollWheel};
use crate::lights::SimulatedLight;
use crate::network::{HostHttpClient, HostWifi};
use crate::panel::SimulatedPanel;
use crate::speaker::LoggedSpeaker;
use crate::system::HostSystem;
use crate::window::Case;

const DEFAULT_CARD: &str = "sim-sd";

/// The apps the image holds, in the order the system app offers them; the first is in
/// front at start.
const APPS: &[Install<Frame>] = &[
    #[cfg(feature = "alarm")]
    alarm::install::<Frame>,
    #[cfg(feature = "weather")]
    weather::install::<Frame>,
    #[cfg(feature = "radar")]
    radar::install::<Frame>,
];

enum Computer {}

impl Hardware for Computer {
    type Panel = SimulatedPanel;
    type Wheel = ScrollWheel;
    type Button = KeyButton;
    type Light = SimulatedLight;
    type Rtc = HostClock;
    type Speaker = LoggedSpeaker;
    type Card = DirectoryCard;
    type Wifi = HostWifi;
    type Http = HostHttpClient;
    type Udp = StdUdpClient;
    type System = HostSystem;

    /// Rust's own default: rustls, unoptimised, needs far more than the ESP32's TLS.
    const NETWORK_STACK_BYTES: usize = 2 * 1024 * 1024;
}

/// The window has the main thread: some systems deliver its events on no other.
fn main() -> Result<(), Fault> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let root = std::env::args().nth(1).map_or_else(|| PathBuf::from(DEFAULT_CARD), PathBuf::from);
    let card = DirectoryCard::open(&root).and_then(with_wifi);
    match &card {
        Ok(_) => log::info!("SD card: {}", root.display()),
        Err(fault) => log::warn!("no SD card: {fault}"),
    }

    let case = Case::default();
    let devices = Devices::<Computer> {
        panel: case.panel.clone(),
        wheel: case.wheel.clone(),
        wheel_button: case.wheel_button.clone(),
        yellow_button: case.yellow_button.clone(),
        long_button: case.long_button.clone(),
        front_light: case.front_light.clone(),
        reading_lamp: case.reading_lamp.clone(),
        rtc: HostClock::default(),
        speaker: LoggedSpeaker,
        sd_card: card,
        wifi: HostWifi,
        https: HostHttpClient::default(),
        udp: StdUdpClient,
        system: HostSystem,
    };
    thread::Builder::new()
        .name("app".into())
        .spawn(move || match app::run(devices, APPS) {
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
