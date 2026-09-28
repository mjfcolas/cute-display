use std::sync::{Arc, Mutex};

pub(crate) struct Latest<T>(Arc<Mutex<T>>);

impl<T: Clone> Latest<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn get(&self) -> T {
        match self.0.lock() {
            Ok(value) => value.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn set(&self, value: T) {
        match self.0.lock() {
            Ok(mut slot) => *slot = value,
            Err(poisoned) => *poisoned.into_inner() = value,
        }
    }
}

impl<T> Clone for Latest<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
