use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolve the on-disk path of a framework package by name, searching upward
/// from the crate manifest so the test helper does not depend on a fixed
/// relative layout.
fn find_package_dir(package_name: &str) -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = Some(manifest_dir.as_path());
    while let Some(d) = dir {
        let candidate =
            d.join("crates").join("kanari-frameworks").join("packages").join(package_name);
        if candidate.is_dir() {
            return candidate.canonicalize().with_context(|| {
                format!("resolve {package_name} package path from {}", candidate.display())
            });
        }
        dir = d.parent();
    }
    // Fall back to the conventional relative location for error reporting.
    anyhow::bail!(
        "Could not locate {package_name} package directory (tried ancestors of {})",
        manifest_dir.display()
    )
}

pub fn kanari_system_package_path() -> Result<String> {
    Ok(move_manifest_local_path(&find_package_dir("kanari-system")?))
}

#[allow(dead_code)]
pub fn kanari_governance_package_path() -> Result<String> {
    Ok(move_manifest_local_path(&find_package_dir("kanari-governance")?))
}

fn move_manifest_local_path(path: &Path) -> String {
    let normalized = path.display().to_string().replace('\\', "/");
    normalized.strip_prefix("//?/").unwrap_or(&normalized).to_string()
}
