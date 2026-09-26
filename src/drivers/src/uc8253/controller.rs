use core::ops::RangeInclusive;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{Input, Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use hal::display::{EpaperDisplay, Frame, Redraw, Refreshed, HEIGHT};
use hal::Fault;

use super::memory::{self, Image, BYTES};
use crate::or_fault::OrFault;

mod command {
    pub const PANEL_SETTING: u8 = 0x00;
    pub const POWER_OFF: u8 = 0x02;
    pub const POWER_ON: u8 = 0x04;
    pub const IMAGE_A: u8 = 0x10;
    pub const REFRESH: u8 = 0x12;
    pub const IMAGE_B: u8 = 0x13;
    pub const VCOM_AND_DATA_INTERVAL: u8 = 0x50;
    pub const PARTIAL_WINDOW: u8 = 0x90;
    pub const PARTIAL_IN: u8 = 0x91;
    pub const PARTIAL_OUT: u8 = 0x92;
    pub const TEMPERATURE_SOURCE: u8 = 0xe0;
    pub const TEMPERATURE: u8 = 0xe5;
}

/// Black/white, waveforms from OTP, gate scan not mirrored, all 416 gate lines. The
/// geometry lives in the undocumented second byte, which is why there is no resolution
/// command.
const PANEL_SETTING: [u8; 2] = [0x17, 0x0d];
/// Bit 0 of the panel setting: held low, the controller stays in soft reset.
const RUNNING: u8 = 0x01;

/// The waveforms are picked by temperature: forcing one selects the fast bank.
const TEMPERATURE_FROM_SENSOR: u8 = 0x00;
const TEMPERATURE_FROM_REGISTER: u8 = 0x02;
const FAST_WAVEFORM_TEMPERATURE: u8 = 0x6e;
const VCOM_CLEAN: u8 = 0x97;
const VCOM_FAST: u8 = 0xd7;

/// Wider partial windows run the waveform and change nothing.
const MAX_FAST_ROWS: usize = 320;
/// Fast refreshes leave ghosts that only a clean one erases.
const FAST_REFRESHES_BETWEEN_CLEAN: u32 = 120;

const POLL_MS: u32 = 10;
const START_TIMEOUT: Duration = Duration::from_secs(2);
const SETTLE_TIMEOUT: Duration = Duration::from_secs(5);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(20);

struct Bus {
    spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
    data_not_command: PinDriver<'static, Output>,
    chip_select: PinDriver<'static, Output>,
}

impl Bus {
    fn send(&mut self, is_data: bool, bytes: &[u8]) -> Result<(), Fault> {
        self.data_not_command.set_level(is_data.into()).or_fault("panel DC")?;
        self.chip_select.set_low().or_fault("panel CS")?;
        let sent = self.spi.write(bytes).or_fault("panel SPI");
        self.chip_select.set_high().or_fault("panel CS")?;
        sent
    }

    fn command(&mut self, command: u8, parameters: &[u8]) -> Result<(), Fault> {
        self.send(false, &[command])?;
        if parameters.is_empty() {
            return Ok(());
        }
        self.send(true, parameters)
    }
}

pub struct Uc8253 {
    bus: Bus,
    reset: PinDriver<'static, Output>,
    /// Low while the controller is working.
    busy: PinDriver<'static, Input>,
    /// Every refresh swaps which image memory holds what is on the glass.
    memories_swapped: bool,
    on_glass: Box<Image>,
    wanted: Box<Image>,
    glass_known: bool,
    fast_refreshes: u32,
}

impl Uc8253 {
    /// The SPI device has no chip select of its own: the controller needs it held
    /// across a command and its data, so it is driven here.
    pub fn new(
        spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
        data_not_command: PinDriver<'static, Output>,
        chip_select: PinDriver<'static, Output>,
        reset: PinDriver<'static, Output>,
        busy: PinDriver<'static, Input>,
    ) -> Result<Self, Fault> {
        let mut panel = Self {
            bus: Bus { spi, data_not_command, chip_select },
            reset,
            busy,
            memories_swapped: false,
            on_glass: Box::new([0xff; BYTES]),
            wanted: Box::new([0xff; BYTES]),
            glass_known: false,
            fast_refreshes: 0,
        };
        panel.initialize()?;
        Ok(panel)
    }

    fn initialize(&mut self) -> Result<(), Fault> {
        for level in [true, false, true] {
            self.reset.set_level(level.into()).or_fault("panel RST")?;
            FreeRtos::delay_ms(20);
        }
        self.wait_until_idle(SETTLE_TIMEOUT, "reset")?;
        let [setting, geometry] = PANEL_SETTING;
        self.bus.command(command::PANEL_SETTING, &[setting & !RUNNING, geometry])?;
        FreeRtos::delay_ms(10);
        self.bus.command(command::PANEL_SETTING, &PANEL_SETTING)?;
        self.power_on()?;
        self.bus.command(command::VCOM_AND_DATA_INTERVAL, &[VCOM_CLEAN])?;
        self.power_off()?;
        // A reset clears both memories; the glass keeps whatever it showed.
        self.memories_swapped = false;
        self.on_glass.fill(0xff);
        self.glass_known = false;
        Ok(())
    }

    /// (holding what is on the glass, receiving what is wanted)
    fn memories(&self) -> (u8, u8) {
        if self.memories_swapped {
            (command::IMAGE_B, command::IMAGE_A)
        } else {
            (command::IMAGE_A, command::IMAGE_B)
        }
    }

    fn refresh_whole(&mut self) -> Result<Refreshed, Fault> {
        let (current, next) = self.memories();
        self.power_on()?;
        // Pixels identical in both memories are not driven, so the current image has
        // to be what is really on the glass, or the old ink stays.
        self.bus.command(current, &self.on_glass[..])?;
        self.bus.command(next, &self.wanted[..])?;
        let took = self.refresh();
        self.power_off()?;
        let took = took?;

        self.on_glass.copy_from_slice(&self.wanted[..]);
        self.glass_known = true;
        self.fast_refreshes = 0;
        Ok(Refreshed::Whole { took })
    }

    fn refresh_rows(&mut self, rows: RangeInclusive<usize>) -> Result<Refreshed, Fault> {
        let window = partial_window(&rows);
        let (current, next) = self.memories();
        self.power_on()?;
        // Each memory in its own partial block: sharing one spills outside the window.
        for (memory, image) in [(current, &self.on_glass), (next, &self.wanted)] {
            self.bus.command(command::PARTIAL_IN, &[])?;
            self.bus.command(command::PARTIAL_WINDOW, &window)?;
            self.bus.command(memory, memory::rows(image, &rows))?;
            self.bus.command(command::PARTIAL_OUT, &[])?;
        }
        self.bus.command(command::TEMPERATURE_SOURCE, &[TEMPERATURE_FROM_REGISTER])?;
        self.bus.command(command::TEMPERATURE, &[FAST_WAVEFORM_TEMPERATURE])?;
        self.bus.command(command::VCOM_AND_DATA_INTERVAL, &[VCOM_FAST])?;
        self.bus.command(command::PARTIAL_IN, &[])?;
        self.bus.command(command::PARTIAL_WINDOW, &window)?;

        let took = self.refresh();

        self.bus.command(command::PARTIAL_OUT, &[])?;
        // The forced temperature persists and would select the fast bank for every
        // later refresh.
        self.bus.command(command::TEMPERATURE_SOURCE, &[TEMPERATURE_FROM_SENSOR])?;
        self.bus.command(command::VCOM_AND_DATA_INTERVAL, &[VCOM_CLEAN])?;
        self.power_off()?;
        let took = took?;

        memory::rows_mut(&mut self.on_glass, &rows).copy_from_slice(memory::rows(&self.wanted, &rows));
        self.fast_refreshes += 1;
        Ok(Refreshed::Columns { count: rows.clone().count() as u16, took })
    }

    fn refresh(&mut self) -> Result<Duration, Fault> {
        self.bus.command(command::REFRESH, &[])?;
        self.wait_until_busy(START_TIMEOUT)
            .ok_or_else(|| Fault::new("panel refresh never started; the image memories are now unknown"))?;
        self.memories_swapped = !self.memories_swapped;
        self.wait_until_idle(REFRESH_TIMEOUT, "refresh")
    }

    fn power_on(&mut self) -> Result<(), Fault> {
        self.bus.command(command::POWER_ON, &[])?;
        if self.wait_until_busy(START_TIMEOUT).is_none() {
            log::warn!("panel: power on did not assert BUSY");
        }
        self.wait_until_idle(SETTLE_TIMEOUT, "power on").map(drop)
    }

    fn power_off(&mut self) -> Result<(), Fault> {
        self.bus.command(command::POWER_OFF, &[])?;
        self.wait_until_idle(SETTLE_TIMEOUT, "power off").map(drop)
    }

    fn wait_until_idle(&self, timeout: Duration, doing: &str) -> Result<Duration, Fault> {
        let start = Instant::now();
        while self.busy.is_low() {
            if start.elapsed() > timeout {
                return Err(Fault::new(format!("panel still busy after {timeout:?} of {doing}")));
            }
            FreeRtos::delay_ms(POLL_MS);
        }
        Ok(start.elapsed())
    }

    /// Without this, a wait straight after a command sees idle before the controller
    /// has picked the command up.
    fn wait_until_busy(&self, timeout: Duration) -> Option<Duration> {
        let start = Instant::now();
        while self.busy.is_high() {
            if start.elapsed() > timeout {
                return None;
            }
            FreeRtos::delay_ms(POLL_MS);
        }
        Some(start.elapsed())
    }
}

impl EpaperDisplay for Uc8253 {
    fn show(&mut self, frame: &Frame, redraw: Redraw) -> Result<Refreshed, Fault> {
        memory::render(frame, &mut self.wanted);
        let clean_due = !self.glass_known || self.fast_refreshes >= FAST_REFRESHES_BETWEEN_CLEAN;
        if redraw == Redraw::Whole || clean_due {
            return self.refresh_whole();
        }
        match memory::changed_rows(&self.on_glass, &self.wanted) {
            None => Ok(Refreshed::Nothing),
            Some(rows) if rows.clone().count() > MAX_FAST_ROWS => self.refresh_whole(),
            Some(rows) => self.refresh_rows(rows),
        }
    }
}

/// Always the full height. The last byte leaves "scan every gate line" off: on, a
/// small window wears a full-width band into the glass.
fn partial_window(rows: &RangeInclusive<usize>) -> [u8; 7] {
    let [first_hi, first_lo] = (*rows.start() as u16).to_be_bytes();
    let [last_hi, last_lo] = (*rows.end() as u16).to_be_bytes();
    [0, (HEIGHT - 1) as u8, first_hi, first_lo, last_hi, last_lo, 0]
}
