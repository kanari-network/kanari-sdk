//! Kanari Framework
//! Core framework implementation for the Kanari blockchain
//! 
//! This framework provides the core functionality for building and 
//! interacting with the Kanari blockchain platform.

use std::path::PathBuf;
use std::env;

/// Get the path to a package in the framework
fn get_package_path(package: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("packages");
    path.push(package);
    path
}

/// Get the path to the Move standard library
pub fn get_stdlib_path() -> PathBuf {
    get_package_path("move-stdlib")
}

/// Get the path to the Kanari system 
pub fn get_kanari_system_path() -> PathBuf {
    get_package_path("kanari-system")
}

/// Get the path to the Kanari framework
pub fn get_framework_path() -> PathBuf {
    get_package_path("kanari-framework")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_path() {
        let path = get_stdlib_path();
        assert!(path.exists(), "Move stdlib path does not exist: {:?}", path);
    }

    #[test]
    fn test_framework_path() {
        let path = get_framework_path();
        assert!(path.exists(), "Framework path does not exist: {:?}", path);
    }
}




