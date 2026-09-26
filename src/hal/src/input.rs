pub trait RotaryEncoder {
    /// Detents turned since the last call, positive clockwise.
    fn take_detents(&mut self) -> i32;
}

pub trait PushButton {
    /// Presses since the last call. Presses are counted even while nobody asks.
    fn take_presses(&mut self) -> u32;
    fn is_held(&self) -> bool;
}
