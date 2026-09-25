use crate::shared::Shared;

/// Counts up and down, and starts again from zero.
#[derive(Clone, Debug, Default)]
pub struct Counter(Shared<i32>);

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, amount: i32) {
        self.0.update(|count| *count = count.saturating_add(amount));
    }

    pub fn reset(&self) {
        self.0.update(|count| *count = 0);
    }

    pub fn count(&self) -> i32 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_both_ways_and_resets() {
        let counter = Counter::new();
        counter.add(5);
        counter.add(-7);
        assert_eq!(counter.count(), -2);
        counter.reset();
        assert_eq!(counter.count(), 0);
    }

    #[test]
    fn saturates_rather_than_wrapping() {
        let counter = Counter::new();
        counter.add(i32::MAX);
        counter.add(1);
        assert_eq!(counter.count(), i32::MAX);
    }
}
