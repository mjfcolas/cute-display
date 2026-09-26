//! Holding the long button is the system's: it opens the system app. So a press of the long
//! button only reaches an app once it is released early enough to be a press.

use core::time::Duration;

use crate::controls::{Control, ControlsSample, Input};

pub const HOLD_TO_SWITCH: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Input(Input),
    Switch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LongButton {
    Up,
    Down { since: Duration, switched: bool },
}

pub struct Gestures {
    long: LongButton,
}

impl Default for Gestures {
    fn default() -> Self {
        Self { long: LongButton::Up }
    }
}

impl Gestures {
    /// `now` is any monotonic time, as long as it is always the same clock.
    pub fn interpret(&mut self, sample: &ControlsSample, now: Duration) -> Vec<Gesture> {
        let mut gestures = Vec::new();
        if sample.detents != 0 {
            gestures.push(Gesture::Input(Input::Turn(sample.detents)));
        }
        for (control, presses) in [(Control::Wheel, sample.wheel.presses), (Control::Yellow, sample.yellow.presses)] {
            gestures.extend((0..presses).map(|_| Gesture::Input(Input::Press(control))));
        }
        self.interpret_long(sample.long.presses, sample.long.held, now, &mut gestures);
        gestures
    }

    /// Presses are counted as the button goes down, so presses that came and went
    /// between two samples still show up in `presses` even though `held` never saw them.
    fn interpret_long(&mut self, presses: u32, held: bool, now: Duration, gestures: &mut Vec<Gesture>) {
        let fresh = LongButton::Down { since: now, switched: false };
        let (short_presses, next) = match (self.long, held) {
            (LongButton::Up, false) => (presses, LongButton::Up),
            (LongButton::Up, true) => (presses.saturating_sub(1), fresh),
            (LongButton::Down { .. }, true) if presses == 0 => (0, self.long),
            (LongButton::Down { switched, .. }, false) => (u32::from(!switched) + presses, LongButton::Up),
            (LongButton::Down { switched, .. }, true) => (u32::from(!switched) + presses.saturating_sub(1), fresh),
        };
        gestures.extend((0..short_presses).map(|_| Gesture::Input(Input::Press(Control::Long))));
        self.long = next;

        if let LongButton::Down { since, switched: false } = self.long {
            if now.saturating_sub(since) >= HOLD_TO_SWITCH {
                self.long = LongButton::Down { since, switched: true };
                gestures.push(Gesture::Switch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::ButtonSample;

    const TICK: Duration = Duration::from_millis(20);

    fn long(presses: u32, held: bool) -> ControlsSample {
        ControlsSample { long: ButtonSample { presses, held }, ..Default::default() }
    }

    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    const LONG_PRESS: Gesture = Gesture::Input(Input::Press(Control::Long));

    #[test]
    fn a_short_long_press_reaches_the_app_on_release() {
        let mut g = Gestures::default();
        assert_eq!(g.interpret(&long(1, true), at(0)), []);
        assert_eq!(g.interpret(&long(0, true), at(500)), []);
        assert_eq!(g.interpret(&long(0, false), at(600)), [LONG_PRESS]);
    }

    #[test]
    fn holding_the_long_button_switches_once_and_nothing_on_release() {
        let mut g = Gestures::default();
        g.interpret(&long(1, true), at(0));
        assert_eq!(g.interpret(&long(0, true), HOLD_TO_SWITCH), [Gesture::Switch]);
        assert_eq!(g.interpret(&long(0, true), HOLD_TO_SWITCH * 3), []);
        assert_eq!(g.interpret(&long(0, false), HOLD_TO_SWITCH * 4), []);
    }

    #[test]
    fn presses_between_two_samples_are_not_lost() {
        let mut g = Gestures::default();
        assert_eq!(g.interpret(&long(2, false), at(0)), [LONG_PRESS, LONG_PRESS]);

        g.interpret(&long(1, true), at(100));
        assert_eq!(g.interpret(&long(1, true), at(200)), [LONG_PRESS], "released and pressed again unseen");
        assert_eq!(g.interpret(&long(0, true), at(200) + HOLD_TO_SWITCH), [Gesture::Switch], "the new press is timed afresh");
    }

    #[test]
    fn other_controls_reach_the_app_at_once() {
        let mut g = Gestures::default();
        let sample = ControlsSample {
            detents: -3,
            wheel: ButtonSample { presses: 1, held: true },
            yellow: ButtonSample { presses: 2, held: false },
            ..Default::default()
        };
        assert_eq!(
            g.interpret(&sample, TICK),
            [
                Gesture::Input(Input::Turn(-3)),
                Gesture::Input(Input::Press(Control::Wheel)),
                Gesture::Input(Input::Press(Control::Yellow)),
                Gesture::Input(Input::Press(Control::Yellow)),
            ]
        );
    }
}
