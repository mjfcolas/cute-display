//! Which refresh the glass gets: a clean one on the whole glass, or a fast one on the
//! rows that changed. Clean ones are kept rare, ghosts accepted.

use core::ops::RangeInclusive;

use hal::display::{Redraw, HEIGHT};

use super::memory::{self, Image};

/// Wider partial windows run the waveform and change nothing.
const MAX_FAST_ROWS: usize = 320;
/// Fast refreshes leave ghosts that only a clean one erases.
const FAST_REFRESHES_BETWEEN_CLEAN: u32 = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    Whole,
    Rows(RangeInclusive<usize>),
    Nothing,
}

#[derive(Default)]
pub struct RefreshPolicy {
    glass_known: bool,
    fast_refreshes: u32,
}

impl RefreshPolicy {
    pub fn plan(&self, redraw: Redraw, on_glass: &Image, wanted: &Image) -> Plan {
        let clean_due = !self.glass_known || self.fast_refreshes >= FAST_REFRESHES_BETWEEN_CLEAN;
        if redraw == Redraw::Whole || clean_due {
            return Plan::Whole;
        }
        match memory::changed_rows(on_glass, wanted) {
            None => Plan::Nothing,
            Some(rows) => Plan::Rows(rows),
        }
    }

    /// The glass now shows what the plan asked for.
    pub fn record(&mut self, plan: &Plan) {
        match plan {
            Plan::Whole => {
                self.glass_known = true;
                self.fast_refreshes = 0;
            }
            Plan::Rows(_) => self.fast_refreshes += 1,
            Plan::Nothing => {}
        }
    }

    /// A reset clears the controller's memories; the glass keeps whatever it showed.
    pub fn forget_glass(&mut self) {
        self.glass_known = false;
    }
}

/// The rows in windows the controller accepts, one after the other.
pub fn fast_windows(rows: &RangeInclusive<usize>) -> impl Iterator<Item = RangeInclusive<usize>> {
    let last = *rows.end();
    rows.clone().step_by(MAX_FAST_ROWS).map(move |first| first..=last.min(first + MAX_FAST_ROWS - 1))
}

/// Always the full height. The last byte leaves "scan every gate line" off: on, a
/// small window wears a full-width band into the glass.
pub fn partial_window(rows: &RangeInclusive<usize>) -> [u8; 7] {
    let [first_hi, first_lo] = (*rows.start() as u16).to_be_bytes();
    let [last_hi, last_lo] = (*rows.end() as u16).to_be_bytes();
    [0, (HEIGHT - 1) as u8, first_hi, first_lo, last_hi, last_lo, 0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uc8253::memory::{BYTES, ROW_BYTES};

    fn white() -> Box<Image> {
        Box::new([0xff; BYTES])
    }

    fn with_rows_changed(rows: RangeInclusive<usize>) -> Box<Image> {
        let mut image = white();
        for row in rows {
            image[row * ROW_BYTES] = 0;
        }
        image
    }

    fn known() -> RefreshPolicy {
        let mut policy = RefreshPolicy::default();
        policy.record(&Plan::Whole);
        policy
    }

    #[test]
    fn an_unknown_glass_is_refreshed_whole() {
        let policy = RefreshPolicy::default();
        assert_eq!(policy.plan(Redraw::Changes, &white(), &with_rows_changed(3..=4)), Plan::Whole);
    }

    #[test]
    fn a_whole_redraw_is_whole() {
        assert_eq!(known().plan(Redraw::Whole, &white(), &white()), Plan::Whole);
    }

    #[test]
    fn changes_are_refreshed_fast_on_their_rows_and_nothing_is_nothing() {
        let policy = known();
        assert_eq!(policy.plan(Redraw::Changes, &white(), &with_rows_changed(3..=4)), Plan::Rows(3..=4));
        assert_eq!(policy.plan(Redraw::Changes, &white(), &white()), Plan::Nothing);
    }

    #[test]
    fn many_rows_are_still_refreshed_fast() {
        assert_eq!(known().plan(Redraw::Changes, &white(), &with_rows_changed(0..=415)), Plan::Rows(0..=415));
    }

    #[test]
    fn rows_too_many_for_one_window_are_split_into_the_widest_windows() {
        assert_eq!(fast_windows(&(10..=300)).collect::<Vec<_>>(), vec![10..=300]);
        assert_eq!(fast_windows(&(0..=415)).collect::<Vec<_>>(), [0..=319, 320..=415]);
        assert_eq!(fast_windows(&(0..=MAX_FAST_ROWS)).collect::<Vec<_>>(), [0..=319, 320..=320]);
    }

    #[test]
    fn enough_fast_refreshes_call_for_a_clean_one() {
        let mut policy = known();
        for _ in 0..FAST_REFRESHES_BETWEEN_CLEAN {
            assert_eq!(policy.plan(Redraw::Changes, &white(), &with_rows_changed(1..=1)), Plan::Rows(1..=1));
            policy.record(&Plan::Rows(1..=1));
        }
        assert_eq!(policy.plan(Redraw::Changes, &white(), &with_rows_changed(1..=1)), Plan::Whole);
        policy.record(&Plan::Whole);
        assert_eq!(policy.plan(Redraw::Changes, &white(), &with_rows_changed(1..=1)), Plan::Rows(1..=1));
    }

    #[test]
    fn a_reset_forgets_the_glass() {
        let mut policy = known();
        policy.forget_glass();
        assert_eq!(policy.plan(Redraw::Changes, &white(), &white()), Plan::Whole);
    }

    #[test]
    fn a_partial_window_spans_the_full_height() {
        assert_eq!(partial_window(&(10..=300)), [0, 239, 0, 10, 1, 44, 0]);
    }
}
