// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use clap::*;
use kanari_common::load_kanari_config;
use move_package::source_package::layout::SourcePackageLayout;
use std::{
    fmt::Display,
    fs::{File, create_dir_all},
    io::Write,
    path::{Path, PathBuf},
};

// --- Kanari Network Fixed Addresses and Names ---
// These are standard addresses required for Move compilation in the Kanari ecosystem.
pub const MOVE_STDLIB_ADDR_NAME: &str = "std";
pub const MOVE_STDLIB_ADDR_VALUE: &str = "0x1";
pub const KANARI_SYSTEM_ADDR_NAME: &str = "kanari_system";
pub const KANARI_SYSTEM_ADDR_VALUE: &str = "0x2";

/// Create a new Move package with name `name` at `path`. If `path` is not provided the package
/// will be created in the directory `name`.
#[derive(Parser)]
#[clap(name = "new")]
pub struct New {
    /// The name of the package to be created.
    pub name: String,
}

impl New {
    /// Tries to load the active address from the kanari config file (used to set the
    /// package's named address). Defaults to 0x1 if config is unavailable or missing address.
    fn get_address_from_config() -> Option<String> {
        match load_kanari_config() {
            Ok(config) => config
                .get("active_address")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        }
    }

    /// Execute the command using default (empty) dependencies, addresses, and custom strings.
    pub fn execute_with_defaults(self, path: Option<PathBuf>) -> anyhow::Result<()> {
        self.execute(
            path,
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
            "",
        )
    }

    /// Main logic to create the package, including manifest, source file, and tests.
    pub fn execute(
        self,
        path: Option<PathBuf>,
        deps: impl IntoIterator<Item = (impl Display, impl Display)>,
        addrs: impl IntoIterator<Item = (impl Display, impl Display)>,
        custom: &str, // anything else that needs to end up being in Move.toml (or empty string)
    ) -> anyhow::Result<()> {
        let Self { name } = self;
        let p: PathBuf;

        // Determine the root path for the new package
        let path: &Path = match path {
            Some(path) => {
                p = path;
                &p
            }
            None => Path::new(&name),
        };

        // 1. Create directory structure
        create_dir_all(path.join(SourcePackageLayout::Sources.path()))?;

        // 2. Create the Move.toml manifest file
        let mut w = std::fs::File::create(path.join(SourcePackageLayout::Manifest.path()))?;

        // 3. Create the initial source file (e.g., my_package/sources/my_package.move)
        let file_path = path
            .join(SourcePackageLayout::Sources.path())
            .join(format!("{}.move", name));
        let mut file = File::create(file_path)?;
        // Initial module declaration
        write!(file, "module {}::{} {{\n\n}}", name, name)?;

        // --- Write [package] section ---
        writeln!(
            w,
            r#"[package]
name = "{name}"
edition = "legacy" # edition = "legacy" to use legacy (pre-2024) Move
# license = ""           # e.g., "MIT", "GPL", "Apache 2.0"
# authors = ["..."]      # e.g., ["Joe Smith (joesmith@noemail.com)", "John Snow (johnsnow@noemail.com)"]"#
        )?;

        // Write custom package dependencies (if any)
        for (dep_name, dep_val) in deps {
            writeln!(w, "{dep_name} = {dep_val}")?;
        }

        // --- Write [dependencies] section ---
        writeln!(
            w,
            r#"
[dependencies]
# Dependencies point to the Kanari SDK for framework and stdlib
KanariSystem = {{ git = "https://github.com/jamesatomc/kanari-sdk_V2.git", subdir = "crates/kanari-frameworks/packages/kanari-system", rev = "main" }}
MoveStdlib = {{ git = "https://github.com/jamesatomc/kanari-sdk_V2.git", subdir = "crates/kanari-frameworks/packages/move-stdlib", rev = "main" }}
# For remote import, use the `{{ git = "...", subdir = "...", rev = "..." }}`.
# Revision can be a branch, a tag, and a commit hash.
# MyRemotePackage = {{ git = "https://some.remote/host.git", subdir = "remote/path", rev = "main" }}

# For local dependencies use `local = path`. Path is relative to the package root
# Local = {{ local = "../path/to" }}

# To resolve a version conflict and force a specific version for dependency
# override use `override = true`
# Override = {{ local = "../conflicting/version", override = true }}"#
        )?;

        // Write custom named addresses (if any)
        for (addr_name, addr_val) in addrs {
            writeln!(w, "{addr_name} = \"{addr_val}\"")?;
        }

        // --- Write [addresses] section ---
        let address = Self::get_address_from_config().unwrap_or_else(|| "0x1".to_string());

        writeln!(
            w,
            r#"
[addresses]
{name} = "{address}" # This package's named address, defaults to active config address
std = "0x1"
kanari_system = "0x2"
# Named addresses will be accessible in Move as `@name`. They're also exported:
# for example, `std = "0x1"` is exported by the Standard Library.
# alice = "0xA11CE"

[dev-dependencies]
# The dev-dependencies section allows overriding dependencies for `--test` and
# `--dev` modes. You can introduce test-only dependencies here.
# Local = {{ local = "../path/to/dev-build" }}

[dev-addresses]
# The dev-addresses section allows overwriting named addresses for the `--test`
# and `--dev` modes.
# alice = "0xB0B""#
        )?;

        // custom addition in the end
        if !custom.is_empty() {
            writeln!(w, "{}", custom)?;
        }

        // 4. Create tests directory and basic test file
        create_dir_all(path.join("tests"))?;

        let test_file_path = path.join("tests").join(format!("{}_tests.move", name));
        let mut test_file = File::create(test_file_path)?;
        write!(
            test_file,
            r#"#[test_only]
module {}::{}_tests {{

}}"#,
            name, name
        )?;

        // 5. Create .gitignore file
        create_gitignore(path)?;

        Ok(())
    }
}

/// Helper function to create the .gitignore file.
fn create_gitignore(project_path: &Path) -> std::io::Result<()> {
    let gitignore_content = r#"# Move build output
build/

# Move cache
.move/

# IDE
.idea/
.vscode/

# OS
.DS_Store
Thumbs.db

# Move coverage and test files
*.coverage
*.test
"#;

    std::fs::write(project_path.join(".gitignore"), gitignore_content)
}
