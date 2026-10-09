//! For tests only: test doubles of the HAL's contracts, and the checks that every
//! implementation of a contract must pass. So a test on a double shows what the hardware
//! would do.
//!
//! A double is a handle: its clones share one device. One clone is given to the code under
//! test. The test keeps another one, to act on the device and to see what it was told.

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
