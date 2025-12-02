use anyhow::{Context, Result};
use std::path::PathBuf;

/// Framework path resolver for Kanari Move packages
pub struct FrameworkPath;

impl FrameworkPath {
    /// Get the workspace root directory
    fn workspace_root() -> PathBuf {
        // Try to find workspace root by looking for Cargo.toml
        let mut current = std::env::current_dir().unwrap_or_default();

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is the workspace root (not a crate)
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if content.contains("[workspace]") {
                        return current;
                    }
                }
            }

            // Move up one directory
            if !current.pop() {
                // Fallback to current directory if we can't find workspace root
                return std::env::current_dir().unwrap_or_default();
            }
        }
    }

    /// Get the path to kanari-system bytecode modules
    pub fn kanari_system_modules() -> PathBuf {
        Self::workspace_root()
            .join("crates")
            .join("kanari-frameworks")
            .join("packages")
            .join("kanari-system")
            .join("build")
            .join("KanariSystem")
            .join("bytecode_modules")
    }

    /// Get the path to move-stdlib bytecode modules
    pub fn move_stdlib_modules() -> PathBuf {
        Self::workspace_root()
            .join("crates")
            .join("kanari-frameworks")
            .join("packages")
            .join("move-stdlib")
            .join("build")
            .join("build")
            .join("MoveStdlib")
            .join("bytecode_modules")
    }

    /// Get the path to stdlib dependencies within kanari-system build
    pub fn stdlib_dependencies() -> PathBuf {
        Self::kanari_system_modules()
            .join("dependencies")
            .join("MoveStdlib")
    }

    /// Find stdlib modules path (tries multiple locations)
    pub fn find_stdlib_modules() -> Option<PathBuf> {
        // Try in-repo move-stdlib build first
        let stdlib_path = Self::move_stdlib_modules();
        if stdlib_path.exists() {
            return Some(stdlib_path);
        }

        // Fallback to dependencies folder
        let deps_path = Self::stdlib_dependencies();
        if deps_path.exists() {
            return Some(deps_path);
        }

        None
    }

    /// Verify that required framework paths exist
    pub fn verify_paths() -> Result<()> {
        let modules_dir = Self::kanari_system_modules();
        if !modules_dir.exists() {
            anyhow::bail!(
                "Bytecode modules directory not found: {}\nBuild the Move package first.",
                modules_dir.display()
            );
        }
        Ok(())
    }

    /// Get all Move module files from a directory
    pub fn get_module_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
        let mut modules = Vec::new();

        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Only include .mv files
            if path.extension().and_then(|s| s.to_str()) == Some("mv") {
                modules.push(path);
            }
        }

        // Sort for deterministic order
        modules.sort();

        Ok(modules)
    }

    /// Read module bytecode from files
    pub fn read_modules(paths: &[PathBuf]) -> Result<Vec<Vec<u8>>> {
        paths
            .iter()
            .map(|path| {
                std::fs::read(path)
                    .with_context(|| format!("Failed to read module: {}", path.display()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_root() {
        let root = FrameworkPath::workspace_root();
        assert!(root.join("Cargo.toml").exists());
    }

    #[test]
    fn test_path_construction() {
        let kanari_path = FrameworkPath::kanari_system_modules();
        assert!(kanari_path.to_string_lossy().contains("kanari-system"));

        let stdlib_path = FrameworkPath::move_stdlib_modules();
        assert!(stdlib_path.to_string_lossy().contains("move-stdlib"));
    }
}
