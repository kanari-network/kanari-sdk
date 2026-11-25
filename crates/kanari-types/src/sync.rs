// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Poison-recovery helpers for process-local locks.
//!
//! Recovery keeps the guarded data intact. Callers remain responsible for any
//! extra cleanup (for example discarding a derived DAG engine) after recovery.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Read-lock a `RwLock`, recovering the data on poison.
pub fn rw_read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    rw_read_or_recover_with(lock, || {})
}

/// Read-lock a `RwLock`, invoking `on_poison` when recovering.
pub fn rw_read_or_recover_with<T, F>(lock: &RwLock<T>, on_poison: F) -> RwLockReadGuard<'_, T>
where
    F: FnOnce(),
{
    lock.read().unwrap_or_else(|error| {
        on_poison();
        PoisonError::into_inner(error)
    })
}

/// Write-lock a `RwLock`, recovering the data on poison.
pub fn rw_write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    rw_write_or_recover_with(lock, || {})
}

/// Write-lock a `RwLock`, invoking `on_poison` when recovering.
pub fn rw_write_or_recover_with<T, F>(lock: &RwLock<T>, on_poison: F) -> RwLockWriteGuard<'_, T>
where
    F: FnOnce(),
{
    lock.write().unwrap_or_else(|error| {
        on_poison();
        PoisonError::into_inner(error)
    })
}

/// Lock a `Mutex`, recovering the data on poison.
pub fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_guards_see_original_data() {
        let lock = RwLock::new(7u64);
        let _panic = std::panic::catch_unwind(|| {
            let _guard = lock.write().unwrap();
            panic!("poison");
        });
        assert!(lock.is_poisoned());
        assert_eq!(*rw_read_or_recover(&lock), 7);
        *rw_write_or_recover(&lock) = 9;
        assert_eq!(*rw_read_or_recover(&lock), 9);
    }

    #[test]
    fn mutex_recovery_preserves_data() {
        let lock = Mutex::new(3u64);
        let _panic = std::panic::catch_unwind(|| {
            let _guard = lock.lock().unwrap();
            panic!("poison");
        });
        assert!(lock.is_poisoned());
        assert_eq!(*mutex_lock_or_recover(&lock), 3);
    }
}
