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

use move_core_types::language_storage::ModuleId;
use move_core_types::move_resource::MoveResource;
use move_binary_format::CompiledModule;
use move_binary_format::errors::VMError;
use move_bytecode_verifier::verifier::verify_module_unmetered;

#[derive(Debug)]
pub enum FrameworkError {
    IoError(io::Error),
    MoveCompileError(String),
    PackageNotFound(String),
    InvalidPackage(String),
    DependencyError(String),
    ModuleError(VMError),
    InvalidModule(String),
}

impl From<io::Error> for FrameworkError {
    fn from(err: io::Error) -> Self {
        FrameworkError::IoError(err)
    }
}

impl From<VMError> for FrameworkError {
    fn from(err: VMError) -> Self {
        FrameworkError::ModuleError(err)
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
    pub name: String,
    pub version: String,
    pub path: Option<PathBuf>,
}

/// Framework package management
pub struct Package {
    pub package_type: PackageType,
    path: PathBuf,
    pub dependencies: HashMap<String, PackageDependency>,
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
        let _content = fs::read_to_string(&toml_path)
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

    /// Parse and verify a compiled Move module
    pub fn parse_module(&self, bytes: &[u8]) -> Result<CompiledModule, FrameworkError> {
        match CompiledModule::deserialize_with_defaults(bytes) {
            Ok(module) => {
                // Verify the module is valid
                verify_module_unmetered(&module).map_err(|e| FrameworkError::ModuleError(e))?;
                Ok(module)
            }
            Err(e) => Err(FrameworkError::InvalidModule(format!("Failed to deserialize module: {}", e))),
        }
    }

    /// Compile Move sources and return compiled modules
    pub fn compile(&self) -> Result<ModuleStore, FrameworkError> {
        let store = ModuleStore::new();
        
        // TODO: Implement actual compilation logic
        // For now, we're just setting up the structure
        
        Ok(store)
    }
}

/// Module storage for compiled Move modules
pub struct ModuleStore {
    modules: HashMap<ModuleId, CompiledModule>,
}

impl ModuleStore {
    /// Create a new empty module store
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Add a compiled module to the store
    pub fn add_module(&mut self, module: CompiledModule) -> Result<(), FrameworkError> {
        let module_id = module.self_id();
        self.modules.insert(module_id, module);
        Ok(())
    }

    /// Get a module by its ID
    pub fn get_module(&self, id: &ModuleId) -> Option<&CompiledModule> {
        self.modules.get(id)
    }
}

/// Package source file information
#[derive(Debug)]
pub struct PackageSourceInfo {
    pub module_name: String,
    pub dependencies: Vec<String>,
    pub has_tests: bool,
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

/// Extensions for working with Move resources
pub trait MoveResourceExt: MoveResource {
    /// Convert the resource to bytes
    fn to_bytes(&self) -> Vec<u8>;
    
    /// Create resource from bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self, FrameworkError> where Self: Sized;
}

#[cfg(test)]
mod tests {
   

    use super::*;

    #[test]
    fn test_package_loading() {
        let mut pkg = Package::new(PackageType::Framework).unwrap();
        assert_eq!(pkg.package_type, PackageType::Framework);
        assert!(pkg.path.exists(), "Package path does not exist: {:?}", pkg.path);
        pkg.load_dependencies().unwrap();
        
        let sources = pkg.get_sources().unwrap();
        assert!(!sources.is_empty(), "No Move source files found");
        
        // Check source metadata
        for (_path, info) in sources {
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