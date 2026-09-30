use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use hal::input::{PushButton, RotaryEncoder};

use crate::shared::lock;

#[derive(Clone, Default)]
pub struct FakeButton(Arc<Mutex<Button>>);

#[derive(Default)]
struct Button {
    presses: u32,
    held: bool,
}

impl FakeButton {
    /// Pressed and released.
    pub fn press(&self) {
        lock(&self.0).presses += 1;
    }

    pub fn set_held(&self, held: bool) {
        lock(&self.0).held = held;
    }
}

impl PushButton for FakeButton {
    fn take_presses(&mut self) -> u32 {
        core::mem::take(&mut lock(&self.0).presses)
    }

    fn is_held(&self) -> bool {
        lock(&self.0).held
    }
}

#[derive(Clone, Default)]
pub struct FakeWheel(Arc<AtomicI32>);

impl FakeWheel {
    /// Positive clockwise.
    pub fn turn(&self, detents: i32) {
        self.0.fetch_add(detents, Ordering::Relaxed);
    }
}

impl RotaryEncoder for FakeWheel {
    fn take_detents(&mut self) -> i32 {
        self.0.swap(0, Ordering::Relaxed)
    }
}

/// Checks a button counts its presses as `PushButton` says; `press` presses it once and
/// releases it.
pub fn check_presses(button: &mut impl PushButton, press: impl Fn()) {
    assert_eq!(button.take_presses(), 0, "never pressed");
    press();
    press();
    assert_eq!(button.take_presses(), 2, "the presses made while nobody asked");
    assert_eq!(button.take_presses(), 0, "each press is taken once");
}

/// Checks a button says whether it is held; `set_held` holds it down or lets it go.
pub fn check_holding(button: &mut impl PushButton, set_held: impl Fn(bool)) {
    set_held(true);
    assert!(button.is_held(), "held down");
    set_held(false);
    assert!(!button.is_held(), "let go");
}

/// Checks a wheel counts its detents as `RotaryEncoder` says; `turn` turns it, positive
/// clockwise.
pub fn check_detents(wheel: &mut impl RotaryEncoder, turn: impl Fn(i32)) {
    assert_eq!(wheel.take_detents(), 0, "never turned");
    turn(2);
    turn(-3);
    assert_eq!(wheel.take_detents(), -1, "the detents turned while nobody asked, clockwise positive");
    assert_eq!(wheel.take_detents(), 0, "each detent is taken once");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_button_keeps_the_contract() {
        let button = FakeButton::default();
        check_presses(&mut button.clone(), || button.press());
        check_holding(&mut button.clone(), |held| button.set_held(held));
    }

    #[test]
    fn a_fake_wheel_keeps_the_contract() {
        let wheel = FakeWheel::default();
        check_detents(&mut wheel.clone(), |detents| wheel.turn(detents));
    }
}
