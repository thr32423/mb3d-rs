//! Locks that one panic does not break for good. A thread that panics while
//! it holds a `Mutex` leaves it "poisoned", and `lock().unwrap()` then
//! panics in every later caller: after one caught render panic every
//! calculation of the session failed until a restart. The data behind these
//! locks (status texts, job lists, caches, buffers that the next
//! calculation fills again) stays usable, so the lock is taken as it is,
//! as the swarm always did.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub trait Locked<T: ?Sized> {
    /// `lock()`, a poisoned lock taken as it is.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub trait RwLocked<T: ?Sized> {
    /// `read()`, a poisoned lock taken as it is.
    fn read_locked(&self) -> RwLockReadGuard<'_, T>;
    /// `write()`, a poisoned lock taken as it is.
    fn write_locked(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLocked<T> for RwLock<T> {
    fn read_locked(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(PoisonError::into_inner)
    }
    fn write_locked(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poisoned_lock_is_taken() {
        let m = std::sync::Arc::new(Mutex::new(5));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("a calculation that failed");
        })
        .join();
        assert!(m.is_poisoned());
        *m.locked() += 1;
        assert_eq!(*m.locked(), 6);
        let r = std::sync::Arc::new(RwLock::new(1));
        let r2 = r.clone();
        let _ = std::thread::spawn(move || {
            let _g = r2.write().unwrap();
            panic!("a calculation that failed");
        })
        .join();
        *r.write_locked() += 1;
        assert_eq!(*r.read_locked(), 2);
    }
}
