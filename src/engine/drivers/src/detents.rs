/// Both edges of A are counted, and the wheel clicks once per quadrature cycle.
const COUNTS_PER_DETENT: i32 = 2;

#[derive(Default)]
pub struct DetentCounter {
    counted: i32,
}

impl DetentCounter {
    /// A half-turned detent stays in the count until it completes.
    pub fn detents_at(&mut self, count: i32) -> i32 {
        let detents = count.saturating_sub(self.counted) / COUNTS_PER_DETENT;
        self.counted = self.counted.saturating_add(detents * COUNTS_PER_DETENT);
        detents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_turned_detent_waits_for_its_other_half() {
        let mut counter = DetentCounter::default();
        assert_eq!(counter.detents_at(3), 1);
        assert_eq!(counter.detents_at(4), 1);
        assert_eq!(counter.detents_at(1), -1);
    }
}
