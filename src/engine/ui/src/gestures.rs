//! A click of the wheel is the system's: it is a `Gesture::System`, never an app's
//! `Input`. The yellow and long buttons are held together to mean something of
//! their own, so a press of either only reaches an app once it is released, and not at
//! all when it was part of that hold.

use core::time::Duration;

use crate::controls::{Button, ButtonSample, ControlsSample, Input};

pub const HOLD_TOGETHER: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Input(Input),
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonState {
    Up,
    Pressed { since: Duration },
    HeldTogether,
}

impl ButtonState {
    /// Presses are counted as the button goes down, so presses that came and went
    /// between two samples still show up in `presses` even though `held` never saw them.
    fn take_released_presses(&mut self, sample: ButtonSample, now: Duration) -> u32 {
        let ButtonSample { presses, held } = sample;
        let fresh = ButtonState::Pressed { since: now };
        let released_now = |state: ButtonState| u32::from(matches!(state, ButtonState::Pressed { .. }));
        let (released, next) = match (*self, held) {
            (ButtonState::Up, false) => (presses, ButtonState::Up),
            (ButtonState::Up, true) => (presses.saturating_sub(1), fresh),
            (_, true) if presses == 0 => (0, *self),
            (state, false) => (released_now(state) + presses, ButtonState::Up),
            (state, true) => (released_now(state) + presses.saturating_sub(1), fresh),
        };
        *self = next;
        released
    }
}

pub struct Gestures {
    yellow: ButtonState,
    long: ButtonState,
}

impl Default for Gestures {
    fn default() -> Self {
        Self { yellow: ButtonState::Up, long: ButtonState::Up }
    }
}

impl Gestures {
    /// `now` is any monotonic time, as long as it is always the same clock.
    pub fn interpret(&mut self, sample: &ControlsSample, now: Duration) -> Vec<Gesture> {
        let mut gestures: Vec<Gesture> = (0..sample.wheel.presses).map(|_| Gesture::System).collect();
        if sample.detents != 0 {
            gestures.push(Gesture::Input(Input::Turn(sample.detents)));
        }
        let yellow = self.yellow.take_released_presses(sample.yellow, now);
        let long = self.long.take_released_presses(sample.long, now);
        for (button, presses) in [(Button::Yellow, yellow), (Button::Long, long)] {
            gestures.extend((0..presses).map(|_| Gesture::Input(Input::Press(button))));
        }
        if let (ButtonState::Pressed { since: yellow }, ButtonState::Pressed { since: long }) = (self.yellow, self.long) {
            if now.saturating_sub(yellow.max(long)) >= HOLD_TOGETHER {
                self.yellow = ButtonState::HeldTogether;
                self.long = ButtonState::HeldTogether;
                gestures.push(Gesture::Input(Input::HoldYellowAndLong));
            }
        }
        gestures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn button(presses: u32, held: bool) -> ButtonSample {
        ButtonSample { presses, held }
    }

    fn long(presses: u32, held: bool) -> ControlsSample {
        ControlsSample { long: button(presses, held), ..Default::default() }
    }

    fn both(yellow: ButtonSample, long: ButtonSample) -> ControlsSample {
        ControlsSample { yellow, long, ..Default::default() }
    }

    const LONG_PRESS: Gesture = Gesture::Input(Input::Press(Button::Long));
    const YELLOW_PRESS: Gesture = Gesture::Input(Input::Press(Button::Yellow));
    const HOLD: Gesture = Gesture::Input(Input::HoldYellowAndLong);

    #[test]
    fn a_click_of_the_wheel_is_the_system_gesture_at_once() {
        let mut g = Gestures::default();
        let clicked = ControlsSample { wheel: button(1, true), ..Default::default() };
        assert_eq!(g.interpret(&clicked, at(0)), [Gesture::System]);
    }

    #[test]
    fn a_press_reaches_the_app_on_release_however_long_it_was_held() {
        let mut g = Gestures::default();
        assert_eq!(g.interpret(&long(1, true), at(0)), []);
        assert_eq!(g.interpret(&long(0, true), at(5000)), []);
        assert_eq!(g.interpret(&long(0, false), at(5100)), [LONG_PRESS]);
    }

    #[test]
    fn presses_between_two_samples_are_not_lost() {
        let mut g = Gestures::default();
        assert_eq!(g.interpret(&long(2, false), at(0)), [LONG_PRESS, LONG_PRESS]);
        g.interpret(&long(1, true), at(100));
        assert_eq!(g.interpret(&long(1, true), at(200)), [LONG_PRESS], "released and pressed again unseen");
    }

    #[test]
    fn holding_yellow_and_long_together_is_one_gesture_and_no_press() {
        let mut g = Gestures::default();
        assert_eq!(g.interpret(&both(button(0, false), button(1, true)), at(0)), []);
        assert_eq!(g.interpret(&both(button(1, true), button(0, true)), at(300)), []);
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), at(300) + HOLD_TOGETHER - at(1)), []);
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), at(300) + HOLD_TOGETHER), [HOLD]);
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), at(5000)), []);
        assert_eq!(g.interpret(&both(button(0, false), button(0, true)), at(5100)), []);
        assert_eq!(g.interpret(&both(button(0, false), button(0, false)), at(5200)), []);
    }

    #[test]
    fn a_button_pressed_again_unseen_during_the_hold_starts_its_second_afresh() {
        let mut g = Gestures::default();
        g.interpret(&both(button(1, true), button(1, true)), at(0));
        assert_eq!(g.interpret(&both(button(1, true), button(0, true)), at(600)), [YELLOW_PRESS], "released and pressed again unseen");
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), HOLD_TOGETHER), []);
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), at(600) + HOLD_TOGETHER), [HOLD]);
    }

    #[test]
    fn after_the_hold_a_new_press_of_one_button_is_a_press_not_another_hold() {
        let mut g = Gestures::default();
        g.interpret(&both(button(1, true), button(1, true)), at(0));
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), HOLD_TOGETHER), [HOLD]);
        assert_eq!(g.interpret(&both(button(0, false), button(0, true)), at(1500)), []);
        assert_eq!(g.interpret(&both(button(1, true), button(0, true)), at(1600)), []);
        assert_eq!(g.interpret(&both(button(0, true), button(0, true)), at(5000)), [], "long is still part of the first hold");
        assert_eq!(g.interpret(&both(button(0, false), button(0, true)), at(5100)), [YELLOW_PRESS]);
    }

    #[test]
    fn let_go_too_soon_both_are_presses() {
        let mut g = Gestures::default();
        g.interpret(&both(button(1, true), button(1, true)), at(0));
        assert_eq!(g.interpret(&both(button(0, false), button(0, false)), at(500)), [YELLOW_PRESS, LONG_PRESS]);
    }

    #[test]
    fn turns_reach_the_app_at_once() {
        let mut g = Gestures::default();
        let sample = ControlsSample { detents: -3, yellow: button(2, false), ..Default::default() };
        assert_eq!(g.interpret(&sample, at(20)), [Gesture::Input(Input::Turn(-3)), YELLOW_PRESS, YELLOW_PRESS]);
    }
}
