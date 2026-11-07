//! Tests for deps.rs cache repair helpers.

use kanari_move_analyzer::deps::{dependency_caches, move_home};

#[test]
fn move_home_respects_move_home_env() {
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
    // point move_home at temp by setting MOVE_HOME
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", dir.path()) };
    let caches = dependency_caches();
    assert_eq!(caches.len(), 1);
    assert!(caches[0].ends_with("a"));
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}

#[test]
fn dependency_caches_empty_when_no_move_home() {
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", "/nonexistent-xyz-123") };
    assert!(dependency_caches().is_empty());
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}
