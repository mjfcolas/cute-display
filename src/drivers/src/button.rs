//! Push buttons wired as switches to ground: high released, low pressed.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Pressed,
    Released,
}

/// A level change counts once it has been read the same this many samples in a row.
pub const STABLE_SAMPLES: u8 = 4;

pub struct Debouncer {
    pressed: bool,
    agreeing: u8,
}

impl Debouncer {
    pub fn new(pressed: bool) -> Self {
        Self { pressed, agreeing: 0 }
    }

    pub fn sample(&mut self, pressed: bool) -> Option<Edge> {
        if pressed == self.pressed {
            self.agreeing = 0;
            return None;
        }
        self.agreeing += 1;
        if self.agreeing < STABLE_SAMPLES {
            return None;
        }
        self.pressed = pressed;
        self.agreeing = 0;
        Some(if pressed { Edge::Pressed } else { Edge::Released })
    }
}

#[cfg(target_os = "espidf")]
pub use sampled::{watch, Button};

#[cfg(target_os = "espidf")]
mod sampled {
    use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread;

    use esp_idf_svc::hal::delay::FreeRtos;
    use esp_idf_svc::hal::gpio::{Input, PinDriver};
    use hal::input::PushButton;
    use hal::Fault;

    use super::{Debouncer, Edge};
    use crate::or_fault::OrFault;

    const SAMPLE_PERIOD_MS: u32 = 10;

    #[derive(Default)]
    struct Tally {
        presses: AtomicU32,
        held: AtomicBool,
    }

    pub struct Button {
        tally: Arc<Tally>,
        reported: u32,
    }

    impl PushButton for Button {
        fn take_presses(&mut self) -> u32 {
            let presses = self.tally.presses.load(Ordering::Relaxed);
            let new = presses.wrapping_sub(self.reported);
            self.reported = presses;
            new
        }

        fn is_held(&self) -> bool {
            self.tally.held.load(Ordering::Relaxed)
        }
    }

    /// Samples the buttons on a thread of their own, so a press is counted even while
    /// whoever reads them is blocked. The pins must be pulled up.
    pub fn watch<const N: usize>(pins: [PinDriver<'static, Input>; N]) -> Result<[Button; N], Fault> {
        let tallies: [Arc<Tally>; N] = core::array::from_fn(|_| Arc::default());
        let mut sampled: Vec<_> = pins
            .into_iter()
            .zip(tallies.iter().cloned())
            .map(|(pin, tally)| (Debouncer::new(pin.is_low()), pin, tally))
            .collect();

        thread::Builder::new()
            .name("buttons".into())
            .stack_size(4 * 1024)
            .spawn(move || loop {
                for (debouncer, pin, tally) in &mut sampled {
                    match debouncer.sample(pin.is_low()) {
                        Some(Edge::Pressed) => {
                            tally.held.store(true, Ordering::Relaxed);
                            tally.presses.fetch_add(1, Ordering::Relaxed);
                        }
                        Some(Edge::Released) => tally.held.store(false, Ordering::Relaxed),
                        None => {}
                    }
                }
                FreeRtos::delay_ms(SAMPLE_PERIOD_MS);
            })
            .or_fault("starting the button sampler")?;

        Ok(tallies.map(|tally| Button { tally, reported: 0 }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounces_shorter_than_the_stable_run_are_ignored() {
        let mut d = Debouncer::new(false);
        for _ in 0..3 {
            for _ in 1..STABLE_SAMPLES {
                assert_eq!(d.sample(true), None);
            }
            assert_eq!(d.sample(false), None);
        }
    }

    #[test]
    fn a_held_press_is_one_press_then_one_release() {
        let mut d = Debouncer::new(false);
        let edges: Vec<_> = (0..20).map(|_| true).chain((0..20).map(|_| false)).filter_map(|p| d.sample(p)).collect();
        assert_eq!(edges, [Edge::Pressed, Edge::Released]);
    }
}
