use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;

use hal::input::{PushButton, RotaryEncoder};

#[derive(Default)]
struct KeyState {
    presses: AtomicU32,
    held: AtomicBool,
}

/// Clones share the key; each counts the presses it has taken.
#[derive(Clone, Default)]
pub struct KeyButton {
    key: Arc<KeyState>,
    reported: u32,
}

impl KeyButton {
    pub fn set_down(&self, down: bool) {
        let was_down = self.key.held.swap(down, Ordering::Relaxed);
        if down && !was_down {
            self.key.presses.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl PushButton for KeyButton {
    fn take_presses(&mut self) -> u32 {
        let presses = self.key.presses.load(Ordering::Relaxed);
        let new = presses.wrapping_sub(self.reported);
        self.reported = presses;
        new
    }

    fn is_held(&self) -> bool {
        self.key.held.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Default)]
pub struct ScrollWheel {
    detents: Arc<AtomicI32>,
}

impl ScrollWheel {
    pub fn turn(&self, detents: i32) {
        self.detents.fetch_add(detents, Ordering::Relaxed);
    }
}

impl RotaryEncoder for ScrollWheel {
    fn take_detents(&mut self) -> i32 {
        self.detents.swap(0, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_kept_down_is_one_press_held_until_released() {
        let window = KeyButton::default();
        let mut app = window.clone();
        for _ in 0..5 {
            window.set_down(true);
        }
        assert_eq!(app.take_presses(), 1);
        assert!(app.is_held());
        window.set_down(false);
        assert!(!app.is_held());
        assert_eq!(app.take_presses(), 0);
    }

    #[test]
    fn presses_made_while_nobody_reads_are_all_counted() {
        let window = KeyButton::default();
        let mut app = window.clone();
        for _ in 0..3 {
            window.set_down(true);
            window.set_down(false);
        }
        assert_eq!(app.take_presses(), 3);
    }

    #[test]
    fn detents_add_up_until_taken() {
        let window = ScrollWheel::default();
        let mut app = window.clone();
        window.turn(1);
        window.turn(1);
        window.turn(-3);
        assert_eq!(app.take_detents(), -1);
        assert_eq!(app.take_detents(), 0);
    }
}
