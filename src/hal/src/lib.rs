//! Contracts between the application and the hardware of the Habity bedside clock.
//!
//! Every contract is blocking and single-owner: scheduling work on threads is the
//! caller's business, not the hardware's.

pub mod audio;
pub mod bus;
pub mod clock;
pub mod display;
pub mod http;
pub mod input;
pub mod light;
pub mod power;
pub mod radio;
pub mod storage;
pub mod system;
pub mod thermometer;
pub mod udp;

mod fault;

pub use fault::Fault;
