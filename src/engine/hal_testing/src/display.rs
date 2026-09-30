use std::sync::{Arc, Mutex};

use hal::display::{EpaperDisplay, Frame, Redraw, Refreshed};
use hal::Fault;

use crate::shared::lock;

/// Keeps every frame it is asked to show, with the redraw asked, and refreshes nothing.
#[derive(Clone, Default)]
pub struct StubPanel(Arc<Mutex<Shown>>);

#[derive(Default)]
struct Shown {
    frames: Vec<(Redraw, Frame)>,
    redraws_taken: usize,
}

impl StubPanel {
    pub fn times_shown(&self) -> usize {
        lock(&self.0).frames.len()
    }

    pub fn last_shown(&self) -> Option<(Redraw, Frame)> {
        lock(&self.0).frames.last().cloned()
    }

    /// The redraws asked since the last time a test took them.
    pub fn take_redraws(&self) -> Vec<Redraw> {
        let mut shown = lock(&self.0);
        let new: Vec<Redraw> = shown.frames.iter().skip(shown.redraws_taken).map(|(redraw, _)| *redraw).collect();
        shown.redraws_taken = shown.frames.len();
        new
    }
}

impl EpaperDisplay for StubPanel {
    fn show(&mut self, frame: &Frame, redraw: Redraw) -> Result<Refreshed, Fault> {
        lock(&self.0).frames.push((redraw, frame.clone()));
        Ok(Refreshed::Nothing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taking_the_redraws_leaves_what_was_shown() {
        let mut panel = StubPanel::default();
        panel.show(&Frame::blank(), Redraw::Whole).unwrap();
        panel.show(&Frame::blank(), Redraw::Changes).unwrap();
        assert_eq!(panel.take_redraws(), [Redraw::Whole, Redraw::Changes]);
        assert_eq!(panel.take_redraws(), []);
        assert_eq!(panel.times_shown(), 2);
        assert_eq!(panel.last_shown().map(|(redraw, _)| redraw), Some(Redraw::Changes));
    }
}
