//! For tests only: test doubles of the HAL's contracts, and the checks every
//! implementation of a contract passes, so that a test on a double says what the
//! hardware would do.
//!
//! A double is a handle: its clones share one device, one handed to the code under test,
//! another kept by the test to act on it and see what it was told.

pub mod audio;
pub mod clock;
pub mod display;
pub mod http;
pub mod input;
pub mod light;
pub mod radio;
pub mod running_rtc;
pub mod steady_clock;
pub mod stepped_clock;
pub mod storage;
pub mod system;
pub mod udp;

mod shared;
