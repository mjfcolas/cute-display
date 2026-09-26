//! The hardware test bench: exercises every part of the board through the HAL and
//! reports what each one says on the panel.

pub mod bench;
pub mod chime;
pub mod report;
pub mod screen;

mod latest;
mod painter;
mod survey;

pub use bench::{Bench, Blocking, Controls, Lights, Sensors};
