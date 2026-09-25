//! The Habity bedside clock: which chip is on which pin. See `docs/hardware.md`.

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::adc::oneshot::config::{AdcChannelConfig, Calibration};
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::{attenuation, ADCCH1, ADCU1};
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver};
use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
use esp_idf_svc::hal::spi::{Dma, SpiDeviceDriver, SpiDriver};
use esp_idf_svc::hal::units::FromValueType;

use drivers::adc_power::{AdcPowerMonitor, SharedAdc};
use drivers::button::{self, Button};
use drivers::ds3231::{Ds3231Clock, Ds3231Thermometer};
use drivers::esp_system::EspSystem;
use drivers::esp_wifi::EspWifiScanner;
use drivers::i2c::I2cMaster;
use drivers::i2s_speaker::I2sSpeaker;
use drivers::ledc_light::{self, LedcLight};
use drivers::pcnt_encoder::PcntEncoder;
use drivers::sdmmc_card::SdmmcCard;
use drivers::uc8253::{memory, Uc8253};
use hal::bus::I2cBus;
use hal::Fault;

/// A whole panel image in one transfer; esp-idf-hal wants a multiple of 4.
const PANEL_DMA_BYTES: usize = 16 * 1024;
const _: () = assert!(PANEL_DMA_BYTES >= memory::BYTES && PANEL_DMA_BYTES.is_multiple_of(4));

pub struct Board {
    pub panel: Uc8253,
    pub wheel: PcntEncoder,
    pub wheel_button: Button,
    pub yellow_button: Button,
    pub long_button: Button,
    pub front_light: LedcLight,
    pub reading_lamp: LedcLight,
    pub speaker: I2sSpeaker,
    pub clock: Ds3231Clock,
    pub thermometer: Ds3231Thermometer,
    /// What answered on the I2C bus at power-up.
    pub i2c_devices: Vec<u8>,
    pub sd_card: Result<SdmmcCard, Fault>,
    pub wifi: EspWifiScanner,
    pub power: AdcPowerMonitor<ADCCH1<ADCU1>>,
    pub system: EspSystem,
}

impl Board {
    pub fn bring_up() -> Result<Self, Fault> {
        let peripherals = Peripherals::take().map_err(Fault::new)?;
        let pins = peripherals.pins;
        let fault = |doing: &'static str| move |e: esp_idf_svc::sys::EspError| Fault::new(format!("{doing}: {e}"));

        let spi = SpiDriver::new(
            peripherals.spi2,
            pins.gpio12,
            pins.gpio11,
            None::<AnyIOPin>,
            &DriverConfig::new().dma(Dma::Auto(PANEL_DMA_BYTES)),
        )
        .map_err(fault("panel SPI"))?;
        let panel = Uc8253::new(
            SpiDeviceDriver::new(spi, None::<AnyOutputPin>, &Config::new().baudrate(10.MHz().into()))
                .map_err(fault("panel SPI"))?,
            PinDriver::output(pins.gpio9).map_err(fault("panel DC"))?,
            PinDriver::output(pins.gpio10).map_err(fault("panel CS"))?,
            PinDriver::output(pins.gpio16).map_err(fault("panel RST"))?,
            PinDriver::input(pins.gpio4, Pull::Floating).map_err(fault("panel BUSY"))?,
        )?;

        let [wheel_button, yellow_button, long_button] = button::watch([
            PinDriver::input(pins.gpio21, Pull::Up).map_err(fault("wheel button"))?,
            PinDriver::input(pins.gpio15, Pull::Up).map_err(fault("yellow button"))?,
            PinDriver::input(pins.gpio44, Pull::Up).map_err(fault("long button"))?,
        ])?;

        let lights = ledc_light::shared_timer(peripherals.ledc.timer0)?;

        let mut i2c = I2cMaster::new(peripherals.i2c0, pins.gpio45, pins.gpio46)?;
        let i2c_devices = i2c.scan();
        log::info!("i2c: {i2c_devices:02x?}");
        // Pulled up on the board; an internal pull would hide a line held low.
        let clock_interrupt = PinDriver::input(pins.gpio3, Pull::Floating).map_err(fault("RTC interrupt"))?;

        let sd_host = SdMmcHostDriver::new_4bits(
            peripherals.sdmmc0,
            pins.gpio5,
            pins.gpio6,
            pins.gpio7,
            pins.gpio8,
            pins.gpio14,
            pins.gpio13,
            None::<AnyIOPin>,
            None::<AnyIOPin>,
            &SdMmcHostConfiguration::new(),
        )
        .map_err(fault("SD host"))?;

        let battery_adc: SharedAdc<ADCCH1<ADCU1>> = Box::leak(Box::new(AdcDriver::new(peripherals.adc1).map_err(fault("ADC1"))?));
        let battery_config = AdcChannelConfig {
            attenuation: attenuation::DB_12,
            calibration: Calibration::Curve,
            ..Default::default()
        };

        Ok(Self {
            panel,
            wheel: PcntEncoder::new(pins.gpio47, pins.gpio48)?,
            wheel_button,
            yellow_button,
            long_button,
            front_light: LedcLight::new(peripherals.ledc.channel1, lights, pins.gpio18)?,
            reading_lamp: LedcLight::new(peripherals.ledc.channel0, lights, pins.gpio17)?,
            speaker: I2sSpeaker::new(peripherals.i2s0, pins.gpio40, pins.gpio38, pins.gpio39, pins.gpio41)?,
            clock: Ds3231Clock::new(i2c.clone(), clock_interrupt),
            thermometer: Ds3231Thermometer::new(i2c),
            i2c_devices,
            sd_card: SdmmcCard::mount(sd_host),
            wifi: EspWifiScanner::new(peripherals.modem, EspSystemEventLoop::take().map_err(fault("event loop"))?)?,
            power: AdcPowerMonitor::new(
                // Fed through ~340 kOhm: any internal pull would swamp it.
                PinDriver::input(pins.gpio1, Pull::Floating).map_err(fault("USB detect"))?,
                AdcChannelDriver::new(battery_adc, pins.gpio2, &battery_config).map_err(fault("battery ADC"))?,
            ),
            system: EspSystem,
        })
    }
}
