//! Buttons and a wheel a computer presses and turns through the console, for tests.

use core::fmt;
use core::time::Duration;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

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

/// Real time, as the computer waiting for the answer sees it: the image takes the controls
/// between refreshes, and a cold whole refresh lasts up to 6.9 s (docs/hardware.md); the
/// computer stops waiting after 10 s (`REPLY_TIMEOUT_S` in tools/link).
const TAKEN_WITHIN: Duration = Duration::from_secs(8);

/// A tap, a hold or a turn waits for the app image to take it; this is the answer when it
/// did not in time.
#[derive(Debug, PartialEq, Eq)]
pub struct NotTaken(Duration);

impl fmt::Display for NotTaken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the app image did not take it within {:?}", self.0)
    }
}

#[derive(Default)]
struct RemoteButtonState {
    presses: AtomicU32,
    taken: AtomicU32,
    held: AtomicBool,
}

impl RemoteButtonState {
    fn is_taken(&self) -> bool {
        self.taken.load(Ordering::Relaxed) == self.presses.load(Ordering::Relaxed)
    }
}

#[derive(Default)]
struct TakenSignal {
    lock: Mutex<()>,
    taken: Condvar,
}

impl TakenSignal {
    fn notify_taken(&self) {
        // Locked, so that a remote about to wait cannot miss it.
        drop(self.lock.lock().unwrap_or_else(PoisonError::into_inner));
        self.taken.notify_all();
    }
}

/// Clones share their state with the buttons and the wheel they give out.
#[derive(Clone)]
pub struct Remote {
    hold_clock: Arc<dyn SteadyClock + Send + Sync>,
    taken_within: Duration,
    taken_signal: Arc<TakenSignal>,
    detents: Arc<AtomicI32>,
    wheel_button: Arc<RemoteButtonState>,
    yellow_button: Arc<RemoteButtonState>,
    long_button: Arc<RemoteButtonState>,
}

impl Remote {
    pub fn new(hold_clock: impl SteadyClock + Send + Sync + 'static) -> Self {
        Self {
            hold_clock: Arc::new(hold_clock),
            taken_within: TAKEN_WITHIN,
            taken_signal: Arc::default(),
            detents: Arc::default(),
            wheel_button: Arc::default(),
            yellow_button: Arc::default(),
            long_button: Arc::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn taken_within(self, real_time: Duration) -> Self {
        Self { taken_within: real_time, ..self }
    }

    pub fn wheel(&self) -> RemoteWheel {
        RemoteWheel { detents: self.detents.clone(), taken_signal: self.taken_signal.clone() }
    }

    pub fn button(&self, button: ButtonName) -> RemoteButton {
        RemoteButton { state: self.state(button).clone(), taken_signal: self.taken_signal.clone() }
    }

    pub fn tap(&self, button: ButtonName) -> Result<(), NotTaken> {
        let state = self.state(button);
        state.presses.fetch_add(1, Ordering::Relaxed);
        self.wait_until_taken(|| state.is_taken())
    }

    /// The buttons are released before the wait.
    pub fn hold(&self, buttons: &[ButtonName], duration: HoldDuration) -> Result<(), NotTaken> {
        for &button in buttons {
            let state = self.state(button);
            state.held.store(true, Ordering::Relaxed);
            state.presses.fetch_add(1, Ordering::Relaxed);
        }
        self.hold_clock.sleep(duration.0);
        for &button in buttons {
            self.state(button).held.store(false, Ordering::Relaxed);
        }
        self.wait_until_taken(|| buttons.iter().all(|&button| self.state(button).is_taken()))
    }

    pub fn turn(&self, clockwise_detents: i32) -> Result<(), NotTaken> {
        self.detents.fetch_add(clockwise_detents, Ordering::Relaxed);
        self.wait_until_taken(|| self.detents.load(Ordering::Relaxed) == 0)
    }

    fn wait_until_taken(&self, taken: impl Fn() -> bool) -> Result<(), NotTaken> {
        let waiting = self.taken_signal.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let (waiting, waited) = self.taken_signal.taken.wait_timeout_while(waiting, self.taken_within, |()| !taken()).unwrap_or_else(PoisonError::into_inner);
        drop(waiting);
        if waited.timed_out() {
            return Err(NotTaken(self.taken_within));
        }
        Ok(())
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
    taken_signal: Arc<TakenSignal>,
}

impl RotaryEncoder for RemoteWheel {
    fn take_detents(&mut self) -> i32 {
        let detents = self.detents.swap(0, Ordering::Relaxed);
        if detents != 0 {
            self.taken_signal.notify_taken();
        }
        detents
    }
}

pub struct RemoteButton {
    state: Arc<RemoteButtonState>,
    taken_signal: Arc<TakenSignal>,
}

impl PushButton for RemoteButton {
    fn take_presses(&mut self) -> u32 {
        let presses = self.state.presses.load(Ordering::Relaxed);
        let new = presses.wrapping_sub(self.state.taken.swap(presses, Ordering::Relaxed));
        if new != 0 {
            self.taken_signal.notify_taken();
        }
        new
    }

    fn is_held(&self) -> bool {
        self.state.held.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use hal_testing::input;

    use super::*;
    use crate::fake_image_controls::{FakeImageControls, Taken};

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
    fn a_tap_is_answered_once_the_image_took_it_and_taken_once() {
        let remote = Remote::new(StubSteadyClock::default());
        let image = FakeImageControls::taking_from(&remote);
        assert_eq!(remote.tap(ButtonName::Yellow), Ok(()));
        assert_eq!(image.taken().yellow_presses, 1);
        assert_eq!(remote.tap(ButtonName::Yellow), Ok(()));
        assert_eq!(image.taken(), Taken { yellow_presses: 2, ..Taken::default() }, "each button its own");
    }

    #[test]
    fn a_turn_is_answered_once_the_image_took_its_detents() {
        let remote = Remote::new(StubSteadyClock::default());
        let image = FakeImageControls::taking_from(&remote);
        assert_eq!(remote.turn(-2), Ok(()));
        assert_eq!(image.taken().detents, -2);
    }

    #[test]
    fn what_the_image_does_not_take_is_answered_so() {
        let remote = Remote::new(StubSteadyClock::default()).taken_within(Duration::from_millis(20));
        let _untouched = (remote.button(ButtonName::Yellow), remote.wheel());
        assert_eq!(remote.turn(0), Ok(()), "nothing to take");
        assert_eq!(remote.tap(ButtonName::Yellow), Err(NotTaken(Duration::from_millis(20))));
        assert!(remote.turn(1).is_err());
    }

    #[test]
    fn a_hold_keeps_its_buttons_down_for_its_duration_then_releases_them() {
        let remote = Remote::new(StubSteadyClock::default());
        let clock = StubSteadyClock { held: Arc::default(), watched: remote.long_button.clone() };
        let held = clock.held.clone();
        let remote = Remote { hold_clock: Arc::new(clock), ..remote };
        let image = FakeImageControls::taking_from(&remote);
        let duration = HoldDuration::new(Duration::from_secs(2)).unwrap();
        assert_eq!(remote.hold(&[ButtonName::Yellow, ButtonName::Long], duration), Ok(()));
        assert_eq!(*held.lock().unwrap(), [true], "held while the hold lasts");
        assert!(!remote.button(ButtonName::Yellow).is_held() && !remote.button(ButtonName::Long).is_held());
        assert_eq!((image.taken().yellow_presses, image.taken().long_presses), (1, 1));
    }

    #[test]
    fn a_hold_lasts_ten_seconds_at_most() {
        assert!(HoldDuration::new(HoldDuration::MAX).is_some());
        assert!(HoldDuration::new(HoldDuration::MAX + Duration::from_millis(1)).is_none());
    }

    #[test]
    fn the_remote_buttons_and_wheel_keep_the_contract() {
        let remote = Remote::new(StubSteadyClock::default()).taken_within(Duration::ZERO);
        input::check_presses(&mut remote.button(ButtonName::Yellow), || drop(remote.tap(ButtonName::Yellow)));
        input::check_detents(&mut remote.wheel(), |detents| drop(remote.turn(detents)));
    }
}
