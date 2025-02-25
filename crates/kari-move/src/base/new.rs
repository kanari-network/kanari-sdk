// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use clap::*;
use move_package::source_package::layout::SourcePackageLayout;
use std::{
    fmt::Display,
    fs::{self, create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
};
use serde_yaml::Value;
// TODO get a stable path to this stdlib
// pub const MOVE_STDLIB_PACKAGE_NAME: &str = "MoveStdlib";
// pub const MOVE_STDLIB_PACKAGE_PATH: &str = "{ \
//     git = \"https://github.com/move-language/move.git\", \
//     subdir = \"language/move-stdlib\", rev = \"main\" \
// }";
pub const MOVE_STDLIB_ADDR_NAME: &str = "std";
pub const MOVE_STDLIB_ADDR_VALUE: &str = "0x1";
pub const KANARI_FRAMEWORK_ADDR_NAME: &str = "kanari_framework";
pub const KANARI_FRAMEWORK_ADDR_VALUE: &str = "0x2";

/// Create a new Move package with name `name` at `path`. If `path` is not provided the package
/// will be created in the directory `name`.
#[derive(Parser)]
#[clap(name = "new")]
pub struct New {
    /// The name of the package to be created.
    pub name: String,
}

impl New {

    fn get_address_from_config() -> Option<String> {
        let home_dir = dirs::home_dir()?;
        let config_path = home_dir.join(".kari").join("network").join("config.yaml");
        
        if !config_path.exists() {
            return None;
        }
    
        let config_str = fs::read_to_string(config_path).ok()?;
        let config: Value = serde_yaml::from_str(&config_str).ok()?;
        
        config.get("address")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches(".enc").to_string())
    }


    pub fn execute_with_defaults(self, path: Option<PathBuf>) -> anyhow::Result<()> {
        self.execute(
            path,
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
            "",
        )
    }

    pub fn execute(
        self,
        path: Option<PathBuf>,
        deps: impl IntoIterator<Item = (impl Display, impl Display)>,
        addrs: impl IntoIterator<Item = (impl Display, impl Display)>,
        custom: &str, // anything else that needs to end up being in Move.toml (or empty string)
    ) -> anyhow::Result<()> {
        // TODO warn on build config flags
        let Self { name } = self;
        let p: PathBuf;
        let path: &Path = match path {
            Some(path) => {
                p = path;
                &p
            }
            None => Path::new(&name),
        };
        create_dir_all(path.join(SourcePackageLayout::Sources.path()))?;
        let mut w = std::fs::File::create(path.join(SourcePackageLayout::Manifest.path()))?;
        let file_path = path
            .join(SourcePackageLayout::Sources.path())
            .join(format!("{}.move", name));
        let mut file = File::create(file_path)?;
        write!(file, "module {}::{} {{\n\n}}", name, name)?;

        writeln!(
            w,
            r#"[package]
name = "{name}"
edition = "legacy" # edition = "legacy" to use legacy (pre-2024) Move
# license = ""           # e.g., "MIT", "GPL", "Apache 2.0"
# authors = ["..."]      # e.g., ["Joe Smith (joesmith@noemail.com)", "John Snow (johnsnow@noemail.com)"]"#
        )?;
        for (dep_name, dep_val) in deps {
            writeln!(w, "{dep_name} = {dep_val}")?;
        }

        writeln!(
            w,
            r#"
[dependencies]
KanariFramework = {{ git = "https://github.com/kanari-network/kanari-sdk.git", subdir = "framework/packages/kanari-framework", rev = "kanari-sdk" }}
MoveStdlib = {{ git = "https://github.com/kanari-network/kanari-sdk.git", subdir = "framework/packages/move-stdlib", rev = "kanari-sdk" }}
# For remote import, use the `{{ git = "...", subdir = "...", rev = "..." }}`.
# Revision can be a branch, a tag, and a commit hash.
# MyRemotePackage = {{ git = "https://some.remote/host.git", subdir = "remote/path", rev = "main" }}

# For local dependencies use `local = path`. Path is relative to the package root
# Local = {{ local = "../path/to" }}

# To resolve a version conflict and force a specific version for dependency
# override use `override = true`
# Override = {{ local = "../conflicting/version", override = true }}"#
        )?;

        // write named addresses
        for (addr_name, addr_val) in addrs {
            writeln!(w, "{addr_name} = \"{addr_val}\"")?;
        }

        let address = Self::get_address_from_config()
            .unwrap_or_else(|| "0x1".to_string());
        
        writeln!(
            w,
            r#"
[addresses]
{name} = "{address}"
std = "0x1"
kanari_framework = "0x2"
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

        // Create tests directory
        create_dir_all(path.join("tests"))?;

        // Create basic test file
        let test_file_path = path.join("tests").join(format!("{}_tests.move", name));
        let mut test_file = File::create(test_file_path)?;
        write!(
            test_file,
            r#"#[test_only]
module {}::{}_tests {{

}}"#,
            name, name
        )?;

        create_gitignore(path)?;

        Ok(())
    }
}

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
