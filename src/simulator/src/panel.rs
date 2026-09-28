use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;

use drivers::uc8253::memory::{self, Image};
use drivers::uc8253::refresh::{fast_windows, Plan, RefreshPolicy};
use hal::display::{EpaperDisplay, Frame, Redraw, Refreshed, HEIGHT, WIDTH};
use hal::Fault;

/// Measured on the device: docs/hardware.md.
const WHOLE_REFRESH: Duration = Duration::from_millis(2600);
const FAST_REFRESH: Duration = Duration::from_millis(350);

/// Every clone shows the same glass; only the one the app draws with refreshes it.
#[derive(Clone, Default)]
pub struct SimulatedPanel {
    glass: Arc<Mutex<Frame>>,
    controller: Arc<Mutex<Controller>>,
}

struct Controller {
    policy: RefreshPolicy,
    memory: Box<Image>,
}

impl Default for Controller {
    fn default() -> Self {
        Self { policy: RefreshPolicy::default(), memory: Box::new([0xff; memory::BYTES]) }
    }
}

impl Controller {
    fn refresh(&mut self, frame: &Frame, redraw: Redraw) -> Refreshed {
        let mut wanted: Box<Image> = Box::new([0xff; memory::BYTES]);
        memory::render(frame, &mut wanted);
        let plan = self.policy.plan(redraw, &self.memory, &wanted);
        self.policy.record(&plan);
        self.memory = wanted;
        match plan {
            Plan::Whole => Refreshed::Whole { took: WHOLE_REFRESH },
            Plan::Rows(rows) => Refreshed::Columns {
                count: u16::try_from(rows.clone().count()).unwrap_or(WIDTH),
                took: FAST_REFRESH * u32::try_from(fast_windows(&rows).count()).unwrap_or(1),
            },
            Plan::Nothing => Refreshed::Nothing,
        }
    }
}

impl SimulatedPanel {
    pub fn glass(&self) -> Frame {
        self.glass.lock().map(|glass| glass.clone()).unwrap_or_default()
    }

    fn put_on_glass(&self, frame: Frame) -> Result<(), Fault> {
        *self.glass.lock().map_err(|_| Fault::new("the glass was left half drawn"))? = frame;
        Ok(())
    }
}

impl EpaperDisplay for SimulatedPanel {
    fn show(&mut self, frame: &Frame, redraw: Redraw) -> Result<Refreshed, Fault> {
        let on_glass = self.glass();
        let refreshed = self.controller.lock().map_err(|_| Fault::new("the controller was left mid-plan"))?.refresh(frame, redraw);
        match refreshed {
            Refreshed::Whole { took } => {
                let flashes = whole_refresh(&on_glass, frame);
                let each = took / u32::try_from(flashes.len()).unwrap_or(1);
                for flash in flashes {
                    self.put_on_glass(flash)?;
                    thread::sleep(each);
                }
            }
            Refreshed::Columns { took, .. } => thread::sleep(took),
            Refreshed::Nothing => {}
        }
        self.put_on_glass(frame.clone())?;
        Ok(refreshed)
    }
}

/// What the glass goes through on the clean waveform: the old image inverted, a few
/// flashes of the whole glass to ink and back to paper, the new image inverted.
fn whole_refresh(on_glass: &Frame, wanted: &Frame) -> Vec<Frame> {
    let solid = |color| {
        let mut frame = Frame::blank();
        let Ok(()) = frame.clear(color);
        frame
    };
    let mut flashes = vec![inverted(on_glass)];
    for _ in 0..3 {
        flashes.push(solid(BinaryColor::On));
        flashes.push(solid(BinaryColor::Off));
    }
    flashes.push(inverted(wanted));
    flashes
}

fn inverted(frame: &Frame) -> Frame {
    let mut inverted = Frame::blank();
    for y in 0..i32::from(HEIGHT) {
        for x in 0..i32::from(WIDTH) {
            inverted.set_ink(x, y, !frame.is_ink(x, y));
        }
    }
    inverted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_columns(columns: &[i32]) -> Frame {
        let mut frame = Frame::blank();
        for &x in columns {
            frame.set_ink(x, 5, true);
        }
        frame
    }

    #[test]
    fn changes_refresh_fast_the_span_that_changed_and_nothing_when_none_did() {
        let mut controller = Controller::default();
        assert!(matches!(controller.refresh(&Frame::blank(), Redraw::Changes), Refreshed::Whole { .. }), "the glass is unknown at first");
        assert_eq!(controller.refresh(&Frame::blank(), Redraw::Changes), Refreshed::Nothing);
        assert_eq!(controller.refresh(&with_columns(&[3, 40]), Redraw::Changes), Refreshed::Columns { count: 38, took: FAST_REFRESH });
    }

    #[test]
    fn changes_across_most_of_the_glass_refresh_fast_in_two_windows_as_the_device_does() {
        let mut controller = Controller::default();
        controller.refresh(&Frame::blank(), Redraw::Whole);
        assert_eq!(controller.refresh(&with_columns(&[0, 400]), Redraw::Changes), Refreshed::Columns { count: 401, took: FAST_REFRESH * 2 });
    }

    #[test]
    fn a_whole_refresh_goes_from_the_old_image_inverted_through_flashes_to_the_new_inverted() {
        let old = Frame::blank();
        let mut new = Frame::blank();
        new.set_ink(10, 10, true);
        let flashes = whole_refresh(&old, &new);
        let first = flashes.first().unwrap();
        let last = flashes.last().unwrap();
        assert!(first.is_ink(0, 0) && first.is_ink(10, 10));
        assert!(!last.is_ink(10, 10) && last.is_ink(0, 0));
        let solid: Vec<bool> = flashes[1..flashes.len() - 1].iter().map(|f| f.is_ink(5, 5)).collect();
        assert_eq!(solid, [true, false, true, false, true, false]);
    }
}
