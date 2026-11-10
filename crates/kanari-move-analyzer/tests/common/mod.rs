//! Shared helpers for the analyzer's integration tests.
//!
//! The test binaries run their tests in parallel, and a few tests mutate
//! the process-wide `MOVE_HOME`. Without a lock those tests race each
//! other (the suite became flaky — it failed on the Windows CI runner).
//! Every test that reads or writes `MOVE_HOME` takes `ENV_LOCK`.

use std::sync::Mutex;

/// Serializes the tests that read or write `MOVE_HOME`.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Take the environment lock (poisoning is not an error here).
pub fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
