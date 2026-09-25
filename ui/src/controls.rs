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

/// What an app receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Turn(i32),
    Press(Control),
}
