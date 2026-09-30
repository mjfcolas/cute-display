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
    use hal_testing::input::{FakeButton, FakeWheel};

    use super::*;

    #[test]
    fn a_sample_takes_what_each_control_did_and_leaves_nothing_for_the_next() {
        let mut controls = Controls {
            wheel: FakeWheel::default(),
            wheel_button: FakeButton::default(),
            yellow_button: FakeButton::default(),
            long_button: FakeButton::default(),
        };
        controls.wheel.turn(-2);
        controls.wheel_button.press();
        (0..3).for_each(|_| controls.yellow_button.press());
        controls.long_button.set_held(true);
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
