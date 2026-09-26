use hal::input::{PushButton, RotaryEncoder};
use ui::controls::{ButtonSample, ControlsSample};

pub(crate) struct Controls<E, B> {
    pub wheel: E,
    pub wheel_button: B,
    pub yellow_button: B,
    pub long_button: B,
}

impl<E: RotaryEncoder, B: PushButton> Controls<E, B> {
    pub fn sample(&mut self) -> ControlsSample {
        let button = |b: &mut B| ButtonSample { presses: b.take_presses(), held: b.is_held() };
        ControlsSample {
            detents: self.wheel.take_detents(),
            wheel: button(&mut self.wheel_button),
            yellow: button(&mut self.yellow_button),
            long: button(&mut self.long_button),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeWheel(i32);
    impl RotaryEncoder for FakeWheel {
        fn take_detents(&mut self) -> i32 {
            std::mem::take(&mut self.0)
        }
    }

    struct FakeButton {
        presses: u32,
        held: bool,
    }
    impl PushButton for FakeButton {
        fn take_presses(&mut self) -> u32 {
            std::mem::take(&mut self.presses)
        }
        fn is_held(&self) -> bool {
            self.held
        }
    }

    #[test]
    fn a_sample_takes_what_each_control_did_and_leaves_nothing_for_the_next() {
        let mut controls = Controls {
            wheel: FakeWheel(-2),
            wheel_button: FakeButton { presses: 1, held: false },
            yellow_button: FakeButton { presses: 3, held: false },
            long_button: FakeButton { presses: 0, held: true },
        };
        assert_eq!(
            controls.sample(),
            ControlsSample {
                detents: -2,
                wheel: ButtonSample { presses: 1, held: false },
                yellow: ButtonSample { presses: 3, held: false },
                long: ButtonSample { presses: 0, held: true },
            }
        );
        let next = controls.sample();
        assert_eq!((next.detents, next.wheel.presses, next.yellow.presses), (0, 0, 0));
        assert!(next.long.held);
    }
}
