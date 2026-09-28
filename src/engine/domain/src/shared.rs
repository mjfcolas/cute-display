use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub(crate) struct Shared<T>(Arc<Mutex<T>>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Copy> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn get(&self) -> T {
        match self.0.lock() {
            Ok(value) => *value,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn update(&self, change: impl FnOnce(&mut T)) {
        match self.0.lock() {
            Ok(mut value) => change(&mut value),
            Err(poisoned) => change(&mut poisoned.into_inner()),
        }
    }
}
