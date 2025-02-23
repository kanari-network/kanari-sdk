//! Kanari Framework
//! Kanari Framework
//! Core framework implementation for the Kanari blockchain
//!
//! This framework provides the core functionality for building and
//! interacting with the Kanari blockchain platform.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Framework error types
#[derive(Debug)]
pub enum FrameworkError {
    IoError(io::Error),
    MoveCompileError(String),
    PackageNotFound(String),
    InvalidPackage(String),
    DependencyError(String),
}

impl From<io::Error> for FrameworkError {
    fn from(err: io::Error) -> Self {
        FrameworkError::IoError(err)
    }
}

/// Framework package types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageType {
    Stdlib,
    System,
    Framework,
}

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

/// Package dependency information
#[derive(Debug, Clone)]
pub struct PackageDependency {
    name: String,
    version: String,
    path: Option<PathBuf>,
}

/// Framework package management
pub struct Package {
    package_type: PackageType,
    path: PathBuf,
    dependencies: HashMap<String, PackageDependency>,
}

impl Package {
    /// Create new package instance
    pub fn new(package_type: PackageType) -> Result<Self, FrameworkError> {
        let path = match package_type {
            PackageType::Stdlib => get_stdlib_path(),
            PackageType::System => get_kanari_system_path(),
            PackageType::Framework => get_framework_path(),
        };

        if !path.exists() {
            return Err(FrameworkError::PackageNotFound(
                path.to_string_lossy().into(),
            ));
        }

        Ok(Self {
            package_type,
            path,
            dependencies: HashMap::new(),
        })
    }

    /// Load package dependencies from Move.toml
    pub fn load_dependencies(&mut self) -> Result<(), FrameworkError> {
        let toml_path = self.path.join("Move.toml");
        let content = fs::read_to_string(&toml_path)
            .map_err(|_| FrameworkError::PackageNotFound("Move.toml not found".into()))?;

        // TODO: Parse TOML and populate dependencies
        Ok(())
    }

    /// Get package source files with metadata
    pub fn get_sources(&self) -> Result<Vec<(PathBuf, PackageSourceInfo)>, FrameworkError> {
        let mut sources = Vec::new();
        let source_dir = self.path.join("sources");

        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "move") {
                let info = PackageSourceInfo::new(&path)?;
                sources.push((path, info));
            }
        }

        Ok(sources)
    }

    /// Build package
    pub fn build(&self) -> Result<(), FrameworkError> {
        // Verify dependencies first
        self.verify_dependencies()?;

        // TODO: Add build logic
        Ok(())
    }

    /// Verify package dependencies
    pub fn verify_dependencies(&self) -> Result<bool, FrameworkError> {
        let deps_path = self.path.join("Move.toml");
        if !deps_path.exists() {
            return Err(FrameworkError::PackageNotFound(
                "Move.toml not found".into(),
            ));
        }
        Ok(true)
    }
}


/// Package source file information
#[derive(Debug)]
pub struct PackageSourceInfo {
    module_name: String,
    dependencies: Vec<String>,
    has_tests: bool,
}

impl PackageSourceInfo {
    fn new(path: &PathBuf) -> Result<Self, FrameworkError> {
        let content = fs::read_to_string(path)
            .map_err(|e| FrameworkError::IoError(e))?;

        // TODO: Parse Move source file for metadata
        Ok(Self {
            module_name: path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            dependencies: Vec::new(),
            has_tests: content.contains("#[test]"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_loading() {
        let mut pkg = Package::new(PackageType::Stdlib).unwrap();
        pkg.load_dependencies().unwrap();
        
        let sources = pkg.get_sources().unwrap();
        assert!(!sources.is_empty(), "No Move source files found");
        
        // Check source metadata
        for (path, info) in sources {
            assert!(!info.module_name.is_empty());
            println!("Module: {}, Has tests: {}", info.module_name, info.has_tests);
        }
    }

    #[test]
    fn test_package_build() {
        let pkg = Package::new(PackageType::Framework).unwrap();
        assert!(pkg.build().is_ok());
    }
}