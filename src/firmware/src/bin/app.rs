use std::io::{self, BufRead};
use std::thread;
use std::time::Duration;

use app::{Devices, Hardware};
use drivers::button::Button;
use drivers::ds3231::Ds3231Clock;
use drivers::esp_http::EspHttpsClient;
use drivers::esp_system::EspSystem;
use drivers::esp_wifi::EspWifiRadio;
use drivers::i2s_speaker::I2sSpeaker;
use drivers::ledc_light::LedcLight;
use drivers::pcnt_encoder::PcntEncoder;
use drivers::sdmmc_card::SdmmcCard;
use drivers::uc8253::Uc8253;
use drivers::udp_socket::StdUdpClient;
use drivers::usb_console;
use esp_idf_svc::hal::delay::FreeRtos;
use hal::Fault;
use maintenance::MaintenanceConsole;

use firmware::board::Board;

firmware::image_description!();

const BUILD: &str = concat!("built ", env!("BUILD_TIME"));

enum Habity {}

impl Hardware for Habity {
    type Panel = Uc8253;
    type Wheel = PcntEncoder;
    type Button = Button;
    type Light = LedcLight;
    type Rtc = Ds3231Clock;
    type Speaker = I2sSpeaker;
    type Card = SdmmcCard;
    type Wifi = EspWifiRadio;
    type Http = EspHttpsClient;
    type Udp = StdUdpClient;
    type System = EspSystem;

    const NETWORK_STACK_BYTES: usize = 24 * 1024;
}

fn main() -> Result<(), Fault> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // The monitor re-attaches after resetting the chip; anything logged before is lost.
    FreeRtos::delay_ms(1500);
    log::info!("Cute Display {}, {BUILD}", app::image::VERSION);

    let board = Board::bring_up()?;
    if let Ok(card) = &board.sd_card {
        start_maintenance(card.clone())?;
    }
    let devices = Devices::<Habity> {
        panel: board.panel,
        wheel: board.wheel,
        wheel_button: board.wheel_button,
        yellow_button: board.yellow_button,
        long_button: board.long_button,
        front_light: board.front_light,
        reading_lamp: board.reading_lamp,
        rtc: board.clock,
        speaker: board.speaker,
        sd_card: board.sd_card,
        wifi: board.wifi,
        https: board.https,
        udp: StdUdpClient,
        system: board.system,
    };
    match app::run(devices, catalog::APPS)? {}
}

/// Serves the SD card to a computer on the USB cable; see the installer's `card` command.
fn start_maintenance(card: SdmmcCard) -> Result<(), Fault> {
    usb_console::listen()?;
    thread::Builder::new()
        .name("maintenance".into())
        .stack_size(16 * 1024)
        .spawn(move || {
            let mut console = MaintenanceConsole::new(card);
            let mut line = String::new();
            loop {
                line.clear();
                let replied = io::stdin()
                    .lock()
                    .read_line(&mut line)
                    .and_then(|_| console.on_line(&line, &mut io::stdout().lock()));
                if let Err(e) = replied {
                    log::warn!("maintenance: {e}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        })
        .map_err(Fault::new)?;
    Ok(())
}
