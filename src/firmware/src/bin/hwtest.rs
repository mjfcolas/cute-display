use esp_idf_svc::hal::delay::FreeRtos;
use hal::Fault;
use hwtest::{Bench, Blocking, Controls, Lights, Sensors};

use firmware::board::Board;

firmware::image_description!();

const BUILD: &str = concat!("built ", env!("BUILD_TIME"));

fn main() -> Result<(), Fault> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // The monitor re-attaches after resetting the chip; anything logged before is lost.
    FreeRtos::delay_ms(1500);
    log::info!("Cute Display {} hardware test, {BUILD}", app::image::VERSION);

    let mut board = Board::bring_up()?;
    match board.clock.registers() {
        Ok(registers) => {
            let hex: Vec<String> = registers.iter().map(|b| format!("{b:02x}")).collect();
            log::info!("DS3231 registers: {}", hex.join(" "));
        }
        Err(fault) => log::warn!("DS3231 registers: {fault}"),
    }
    let bench = Bench::start(
        Controls {
            wheel: Box::new(board.wheel),
            wheel_button: Box::new(board.wheel_button),
            yellow_button: Box::new(board.yellow_button),
            long_button: Box::new(board.long_button),
        },
        Lights { front: Box::new(board.front_light), reading_lamp: Box::new(board.reading_lamp) },
        Sensors {
            clock: Box::new(board.clock),
            thermometer: Box::new(board.thermometer),
            i2c_devices: board.i2c_devices,
            storage: board.sd_card.map(|card| Box::new(card) as _),
            power: Box::new(board.power),
            system: Box::new(board.system),
        },
        Blocking { display: board.panel, speaker: board.speaker, wifi: board.wifi },
        BUILD,
    )?;
    bench.run()
}
