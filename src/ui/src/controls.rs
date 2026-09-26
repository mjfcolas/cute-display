//! The controls on the case: a wheel that turns and presses, a yellow button and a long
//! button.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Wheel,
    Yellow,
    Long,
}

/// What one button did since the previous sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonSample {
    pub presses: u32,
    pub held: bool,
}

/// What the controls did since the previous sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlsSample {
    /// Positive clockwise.
    pub detents: i32,
    pub wheel: ButtonSample,
    pub yellow: ButtonSample,
    pub long: ButtonSample,
}

impl ControlsSample {
    /// Somebody did something to a control: a detent, a press, a button held.
    pub fn is_touch(&self) -> bool {
        let buttons = [self.wheel, self.yellow, self.long];
        self.detents != 0 || buttons.iter().any(|b| b.presses > 0 || b.held)
    }
}

/// What an app receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Turn(i32),
    Press(Control),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_control_used_is_a_touch_and_nothing_is_not() {
        assert!(!ControlsSample::default().is_touch());
        assert!(ControlsSample { detents: -1, ..Default::default() }.is_touch());
        assert!(ControlsSample { yellow: ButtonSample { presses: 1, held: false }, ..Default::default() }.is_touch());
        assert!(ControlsSample { long: ButtonSample { presses: 0, held: true }, ..Default::default() }.is_touch());
    }
}
