//! The case on the computer's screen: the glass in a window, with the lights and the keys
//! in a strip under it; the controls on the keyboard and the mouse.

use std::convert::Infallible;

use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH};
use hal::light::{Brightness, DimmableLight};
use hal::Fault;
use minifb::{Key, KeyRepeat, MouseButton, Scale, Window, WindowOptions};

use crate::controls::{KeyButton, ScrollWheel};
use crate::lights::SimulatedLight;
use crate::panel::SimulatedPanel;

const INK: u32 = 0x20_20_20;
const PAPER: u32 = 0xd8_d4_c8;
/// The front light is warm.
const LIT_PAPER: u32 = 0xff_f3_d6;

const STRIP: u32 = 0x3a_3a_3a;
const STRIP_TEXT: u32 = 0xe0_e0_e0;
const KEYS: &str = "arrows/scroll turn · left click press · right yellow · space long";

const COLUMNS: usize = VISIBLE_WIDTH as usize;
const ROWS: usize = HEIGHT as usize;
const STRIP_ROWS: usize = 26;

/// What the window shows and what it moves. Clones of each part go to the app.
#[derive(Default)]
pub struct Case {
    pub panel: SimulatedPanel,
    pub wheel: ScrollWheel,
    pub wheel_button: KeyButton,
    pub yellow_button: KeyButton,
    pub long_button: KeyButton,
    pub front_light: SimulatedLight,
    pub reading_lamp: SimulatedLight,
}

/// Until the window is closed.
pub fn show(case: &Case) -> Result<(), Fault> {
    let options = WindowOptions { scale: Scale::X2, ..WindowOptions::default() };
    let mut window = Window::new("cute-display", COLUMNS, ROWS + STRIP_ROWS, options).map_err(Fault::new)?;
    window.set_target_fps(60);
    let mut pixels = vec![PAPER; COLUMNS * (ROWS + STRIP_ROWS)];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        case.wheel_button.set_down(window.get_mouse_down(MouseButton::Left));
        case.yellow_button.set_down(window.get_mouse_down(MouseButton::Right));
        case.long_button.set_down(window.is_key_down(Key::Space));
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Right => case.wheel.turn(1),
                Key::Left => case.wheel.turn(-1),
                _ => {}
            }
        }
        // Scrolling down turns the wheel the way → does.
        if let Some((_, scrolled)) = window.get_scroll_wheel() {
            case.wheel.turn(if scrolled < 0.0 { 1 } else { -1 });
        }

        let (glass, strip) = pixels.split_at_mut(COLUMNS * ROWS);
        paint(&case.panel.glass(), case.front_light.brightness(), glass);
        label(&mut Strip(strip), case.front_light.brightness(), case.reading_lamp.brightness());
        window.update_with_buffer(&pixels, COLUMNS, ROWS + STRIP_ROWS).map_err(Fault::new)?;
    }
    Ok(())
}

fn paint(glass: &Frame, front_light: Brightness, pixels: &mut [u32]) {
    let paper = if front_light > Brightness::OFF { LIT_PAPER } else { PAPER };
    for (n, pixel) in pixels.iter_mut().enumerate() {
        let (x, y) = ((n % COLUMNS) as i32, (n / COLUMNS) as i32);
        *pixel = if glass.is_ink(x, y) { INK } else { paper };
    }
}

fn label(strip: &mut Strip<'_>, front_light: Brightness, reading_lamp: Brightness) {
    let lights = format!("front light {} % · reading lamp {} %", front_light.as_percent(), reading_lamp.as_percent());
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let Ok(()) = strip.clear(BinaryColor::Off);
    for (line, text) in [lights.as_str(), KEYS].into_iter().enumerate() {
        let at = Point::new(4, 3 + 11 * line as i32);
        let Ok(_) = Text::with_baseline(text, at, style, Baseline::Top).draw(strip);
    }
}

/// The strip's rows of the window's pixels.
struct Strip<'a>(&'a mut [u32]);

impl OriginDimensions for Strip<'_> {
    fn size(&self) -> Size {
        Size::new(COLUMNS as u32, STRIP_ROWS as u32)
    }
}

impl DrawTarget for Strip<'_> {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(&mut self, pixels: I) -> Result<(), Infallible> {
        for Pixel(at, color) in pixels {
            let (Ok(x), Ok(y)) = (usize::try_from(at.x), usize::try_from(at.y)) else {
                continue;
            };
            if let Some(pixel) = self.0.get_mut(y * COLUMNS + x).filter(|_| x < COLUMNS) {
                *pixel = if color.is_on() { STRIP_TEXT } else { STRIP };
            }
        }
        Ok(())
    }
}
