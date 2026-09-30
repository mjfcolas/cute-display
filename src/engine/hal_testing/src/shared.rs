use std::sync::{Mutex, MutexGuard, PoisonError};

/// A test that panicked while holding the lock has already failed; what it left is still
/// worth reading.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
