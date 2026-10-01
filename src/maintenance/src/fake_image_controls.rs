//! The app image's controls for the remote's tests: in a thread of their own, they take
//! what the remote does until dropped.

use core::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use hal::input::{PushButton, RotaryEncoder};

use crate::remote::{ButtonName, Remote};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    pub wheel_presses: u32,
    pub yellow_presses: u32,
    pub long_presses: u32,
    pub detents: i32,
}

pub struct FakeImageControls {
    taken: Arc<Mutex<Taken>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeImageControls {
    pub fn taking_from(remote: &Remote) -> Self {
        let mut wheel_button = remote.button(ButtonName::WheelButton);
        let mut yellow = remote.button(ButtonName::Yellow);
        let mut long = remote.button(ButtonName::Long);
        let mut wheel = remote.wheel();
        let (taken, stop) = (Arc::new(Mutex::new(Taken::default())), Arc::new(AtomicBool::new(false)));
        let (image, stopped) = (taken.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                {
                    let mut taken = image.lock().unwrap();
                    taken.wheel_presses += wheel_button.take_presses();
                    taken.yellow_presses += yellow.take_presses();
                    taken.long_presses += long.take_presses();
                    taken.detents += wheel.take_detents();
                }
                thread::sleep(Duration::from_millis(1));
            }
        });
        Self { taken, stop, thread: Some(thread) }
    }

    pub fn taken(&self) -> Taken {
        *self.taken.lock().unwrap()
    }
}

impl Drop for FakeImageControls {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
