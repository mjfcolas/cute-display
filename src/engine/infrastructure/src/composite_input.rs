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
    use hal_testing::input::{self, FakeButton, FakeWheel};

    use super::*;

    #[test]
    fn a_composite_button_keeps_the_contract_whichever_is_used() {
        let (first, second) = (FakeButton::default(), FakeButton::default());
        let mut button = CompositeButton::new(first.clone(), second.clone());
        input::check_presses(&mut button, || first.press());
        input::check_presses(&mut button, || second.press());
        input::check_holding(&mut button, |held| first.set_held(held));
        input::check_holding(&mut button, |held| second.set_held(held));
    }

    #[test]
    fn a_composite_button_counts_the_presses_of_both_once() {
        let (first, second) = (FakeButton::default(), FakeButton::default());
        let mut button = CompositeButton::new(first.clone(), second.clone());
        first.press();
        second.press();
        second.press();
        assert_eq!(button.take_presses(), 3);
        assert_eq!(button.take_presses(), 0);
    }

    #[test]
    fn a_composite_button_is_held_while_either_is() {
        let held = |first: bool, second: bool| {
            let (a, b) = (FakeButton::default(), FakeButton::default());
            a.set_held(first);
            b.set_held(second);
            CompositeButton::new(a, b).is_held()
        };
        assert!(held(true, false) && held(false, true) && held(true, true));
        assert!(!held(false, false));
    }

    #[test]
    fn a_composite_wheel_keeps_the_contract_whichever_is_turned() {
        let (first, second) = (FakeWheel::default(), FakeWheel::default());
        let mut wheel = CompositeWheel::new(first.clone(), second.clone());
        input::check_detents(&mut wheel, |detents| first.turn(detents));
        input::check_detents(&mut wheel, |detents| second.turn(detents));
    }

    #[test]
    fn a_composite_wheel_adds_the_detents_of_both_and_stops_at_what_a_number_holds() {
        let (first, second) = (FakeWheel::default(), FakeWheel::default());
        let mut wheel = CompositeWheel::new(first.clone(), second.clone());
        first.turn(2);
        second.turn(-5);
        assert_eq!(wheel.take_detents(), -3);
        assert_eq!(wheel.take_detents(), 0);
        first.turn(2);
        second.turn(i32::MAX);
        assert_eq!(wheel.take_detents(), i32::MAX);
    }
}
