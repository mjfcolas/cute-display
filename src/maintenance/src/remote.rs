//! Buttons and a wheel a computer presses and turns through the console, for tests.

use core::time::Duration;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;

use hal::input::{PushButton, RotaryEncoder};
use hal::steady::SteadyClock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonName {
    WheelButton,
    Yellow,
    Long,
}

impl ButtonName {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "wheel" => Some(Self::WheelButton),
            "yellow" => Some(Self::Yellow),
            "long" => Some(Self::Long),
            _ => None,
        }
    }
}

/// Long enough for any gesture, short enough that a hold cannot keep the console.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldDuration(Duration);

impl HoldDuration {
    pub const MAX: Duration = Duration::from_secs(10);

    pub fn new(duration: Duration) -> Option<Self> {
        (duration <= Self::MAX).then_some(Self(duration))
    }
}

#[derive(Default)]
struct RemoteButtonState {
    presses: AtomicU32,
    held: AtomicBool,
}

/// Clones share their state with the buttons and the wheel they give out.
#[derive(Clone)]
pub struct Remote {
    hold_clock: Arc<dyn SteadyClock + Send + Sync>,
    detents: Arc<AtomicI32>,
    wheel_button: Arc<RemoteButtonState>,
    yellow_button: Arc<RemoteButtonState>,
    long_button: Arc<RemoteButtonState>,
}

impl Remote {
    pub fn new(hold_clock: impl SteadyClock + Send + Sync + 'static) -> Self {
        Self {
            hold_clock: Arc::new(hold_clock),
            detents: Arc::default(),
            wheel_button: Arc::default(),
            yellow_button: Arc::default(),
            long_button: Arc::default(),
        }
    }

    pub fn wheel(&self) -> RemoteWheel {
        RemoteWheel { detents: self.detents.clone() }
    }

    pub fn button(&self, button: ButtonName) -> RemoteButton {
        RemoteButton { state: self.state(button).clone(), reported: 0 }
    }

    pub fn tap(&self, button: ButtonName) {
        self.state(button).presses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hold(&self, buttons: &[ButtonName], duration: HoldDuration) {
        for &button in buttons {
            let state = self.state(button);
            state.held.store(true, Ordering::Relaxed);
            state.presses.fetch_add(1, Ordering::Relaxed);
        }
        self.hold_clock.sleep(duration.0);
        for &button in buttons {
            self.state(button).held.store(false, Ordering::Relaxed);
        }
    }

    pub fn turn(&self, clockwise_detents: i32) {
        self.detents.fetch_add(clockwise_detents, Ordering::Relaxed);
    }

    fn state(&self, button: ButtonName) -> &Arc<RemoteButtonState> {
        match button {
            ButtonName::WheelButton => &self.wheel_button,
            ButtonName::Yellow => &self.yellow_button,
            ButtonName::Long => &self.long_button,
        }
    }
}

pub struct RemoteWheel {
    detents: Arc<AtomicI32>,
}

impl RotaryEncoder for RemoteWheel {
    fn take_detents(&mut self) -> i32 {
        self.detents.swap(0, Ordering::Relaxed)
    }
}

pub struct RemoteButton {
    state: Arc<RemoteButtonState>,
    reported: u32,
}

impl PushButton for RemoteButton {
    fn take_presses(&mut self) -> u32 {
        let presses = self.state.presses.load(Ordering::Relaxed);
        let new = presses.wrapping_sub(self.reported);
        self.reported = presses;
        new
    }

    fn is_held(&self) -> bool {
        self.state.held.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Instant;

    use hal_testing::input;

    use super::*;

    #[derive(Default)]
    struct StubSteadyClock {
        held: Arc<Mutex<Vec<bool>>>,
        watched: Arc<RemoteButtonState>,
    }

    impl SteadyClock for StubSteadyClock {
        fn now(&self) -> Instant {
            Instant::now()
        }
        fn sleep(&self, _: Duration) {
            self.held.lock().unwrap().push(self.watched.held.load(Ordering::Relaxed));
        }
    }

    #[test]
    fn taps_are_taken_once_each_button_its_own() {
        let remote = Remote::new(StubSteadyClock::default());
        let mut yellow = remote.button(ButtonName::Yellow);
        let mut long = remote.button(ButtonName::Long);
        remote.tap(ButtonName::Yellow);
        remote.tap(ButtonName::Yellow);
        assert_eq!(yellow.take_presses(), 2);
        assert_eq!(yellow.take_presses(), 0);
        assert_eq!(long.take_presses(), 0);
        assert!(!yellow.is_held());
    }

    #[test]
    fn a_hold_keeps_its_buttons_down_for_its_duration_then_releases_them() {
        let remote = Remote::new(StubSteadyClock::default());
        let clock = StubSteadyClock { held: Arc::default(), watched: remote.long_button.clone() };
        let held = clock.held.clone();
        let remote = Remote { hold_clock: Arc::new(clock), ..remote };
        let mut long = remote.button(ButtonName::Long);
        remote.hold(&[ButtonName::Yellow, ButtonName::Long], HoldDuration::new(Duration::from_secs(2)).unwrap());
        assert_eq!(*held.lock().unwrap(), [true], "held while the hold lasts");
        assert!(!long.is_held());
        assert_eq!(long.take_presses(), 1);
    }

    #[test]
    fn a_hold_lasts_ten_seconds_at_most() {
        assert!(HoldDuration::new(HoldDuration::MAX).is_some());
        assert!(HoldDuration::new(HoldDuration::MAX + Duration::from_millis(1)).is_none());
    }

    #[test]
    fn the_remote_buttons_and_wheel_keep_the_contract() {
        let remote = Remote::new(StubSteadyClock::default());
        input::check_presses(&mut remote.button(ButtonName::Yellow), || remote.tap(ButtonName::Yellow));
        input::check_detents(&mut remote.wheel(), |detents| remote.turn(detents));
    }
}
