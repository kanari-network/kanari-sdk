//! Tests for deps.rs cache repair helpers.
//!
//! These avoid mutating the process-wide `MOVE_HOME` where possible: the
//! test binary runs tests in parallel, so env mutation races with other
//! tests (which previously made this suite flaky). The few tests that do
//! touch the environment take `ENV_LOCK` so they cannot overlap.

use kanari_move_analyzer::deps::{dependency_caches, dependency_caches_in, move_home};

mod common;

#[test]
fn move_home_respects_move_home_env() {
    let _guard = common::lock_env();
    // Save old value
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", "/tmp/custom-move-home") };
    assert_eq!(
        move_home(),
        std::path::PathBuf::from("/tmp/custom-move-home")
    );
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}

#[test]
fn dependency_caches_skips_non_git_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a").join(".git")).unwrap();
    std::fs::create_dir_all(dir.path().join("b")).unwrap();
    std::fs::write(dir.path().join("b").join("file.txt"), "x").unwrap();
    let caches = dependency_caches_in(dir.path());
    assert_eq!(caches.len(), 1);
    assert!(caches[0].ends_with("a"));
}

#[test]
fn dependency_caches_empty_when_no_move_home() {
    let caches = dependency_caches_in(std::path::Path::new("/nonexistent-xyz-123"));
    assert!(caches.is_empty());
}

#[test]
fn dependency_caches_are_under_move_home() {
    // The public entry point resolves MOVE_HOME; every cache it reports must
    // live under that directory. (Comparing the two lists directly is flaky:
    // the analyzer repairs `~/.move` concurrently.)
    let _guard = common::lock_env();
    let home = move_home();
    for c in dependency_caches() {
        assert!(c.starts_with(&home), "{c:?} is not under {home:?}");
    }
}
