use hal::input::{PushButton, RotaryEncoder};

pub struct CompositeButton<A, B> {
    first: A,
    second: B,
}

impl<A: PushButton, B: PushButton> CompositeButton<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A: PushButton, B: PushButton> PushButton for CompositeButton<A, B> {
    fn take_presses(&mut self) -> u32 {
        self.first.take_presses().saturating_add(self.second.take_presses())
    }

    fn is_held(&self) -> bool {
        self.first.is_held() || self.second.is_held()
    }
}

pub struct CompositeWheel<A, B> {
    first: A,
    second: B,
}

impl<A: RotaryEncoder, B: RotaryEncoder> CompositeWheel<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A: RotaryEncoder, B: RotaryEncoder> RotaryEncoder for CompositeWheel<A, B> {
    fn take_detents(&mut self) -> i32 {
        self.first.take_detents().saturating_add(self.second.take_detents())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Button {
        presses: u32,
        held: bool,
    }

    impl PushButton for Button {
        fn take_presses(&mut self) -> u32 {
            std::mem::take(&mut self.presses)
        }
        fn is_held(&self) -> bool {
            self.held
        }
    }

    struct Wheel(i32);

    impl RotaryEncoder for Wheel {
        fn take_detents(&mut self) -> i32 {
            std::mem::take(&mut self.0)
        }
    }

    #[test]
    fn a_composite_button_counts_the_presses_of_both_once() {
        let mut button = CompositeButton::new(Button { presses: 1, held: false }, Button { presses: 2, held: false });
        assert_eq!(button.take_presses(), 3);
        assert_eq!(button.take_presses(), 0);
    }

    #[test]
    fn a_composite_button_is_held_while_either_is() {
        let held = |first, second| CompositeButton::new(Button { presses: 0, held: first }, Button { presses: 0, held: second }).is_held();
        assert!(held(true, false) && held(false, true) && held(true, true));
        assert!(!held(false, false));
    }

    #[test]
    fn a_composite_wheel_adds_the_detents_of_both_and_stops_at_what_a_number_holds() {
        let mut wheel = CompositeWheel::new(Wheel(2), Wheel(-5));
        assert_eq!(wheel.take_detents(), -3);
        assert_eq!(wheel.take_detents(), 0);
        assert_eq!(CompositeWheel::new(Wheel(2), Wheel(i32::MAX)).take_detents(), i32::MAX);
    }
}
