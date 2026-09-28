pub trait RotaryEncoder {
    /// Positive clockwise.
    fn take_detents(&mut self) -> i32;
}

pub trait PushButton {
    /// Presses are counted even while nobody asks.
    fn take_presses(&mut self) -> u32;
    fn is_held(&self) -> bool;
}
