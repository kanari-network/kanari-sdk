// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod build;
pub mod call;
pub mod docgen;
pub mod new;
pub mod publish;
pub mod test;

use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_package::source_package::layout::SourcePackageLayout;
use move_stdlib_natives::{GasParameters, NurseryGasParameters, all_natives, nursery_natives};
use move_vm_runtime::native_functions::NativeFunction;
use std::path::PathBuf;

use clap::Subcommand;

type NativeFunctionRecord = (AccountAddress, Identifier, Identifier, NativeFunction);

/// Top-level `move` subcommands supported by the kanari CLI.
#[derive(Subcommand)]
pub enum MoveCommand {
    /// Build the current Move package
    Build(build::Build),
    /// Create a new Move package
    New(new::New),
    /// Run Move unit tests
    Test(test::Test),
    /// Generate Move docs
    Docgen(docgen::Docgen),
    /// Publish Move module to blockchain
    Publish(publish::Publish),
    /// Call Move function on blockchain
    Call(call::Call),
}

impl MoveCommand {
    /// Execute the selected Move subcommand. This provides a thin dispatch
    /// layer that constructs a default `BuildConfig` where required.
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            MoveCommand::Build(b) => {
                let config = move_package::BuildConfig::default();
                b.execute(None, config)
            }
            MoveCommand::New(n) => n.execute_with_defaults(None),
            MoveCommand::Test(t) => {
                let config = move_package::BuildConfig::default();
                // Construct standard library natives so native functions used by Move
                // packages (e.g., stdlib and unit_test helpers) are available to the VM
                let std_addr = AccountAddress::from_hex_literal("0x1").unwrap();
                let std_natives = all_natives(std_addr, GasParameters::zeros())
                    .into_iter()
                    .chain(nursery_natives(
                        false,
                        std_addr,
                        NurseryGasParameters::zeros(),
                    ));

                // Construct kanari crypto/system natives (registered under package address 0x2)
                let system_addr = AccountAddress::from_hex_literal("0x2").unwrap();
                let crypto_natives =
                    kanari_crypto::move_natives::all_natives(system_addr).into_iter();

                // Merge all natives and pass into test runner
                let natives = std_natives.chain(crypto_natives).collect();
                t.execute(None, config, natives, None)
            }
            MoveCommand::Docgen(d) => {
                let config = move_package::BuildConfig::default();
                d.execute(None, config)
            }
            MoveCommand::Publish(p) => {
                let config = move_package::BuildConfig::default();
                p.execute(None, config)
            }
            MoveCommand::Call(c) => c.execute(),
        }
    }
}
pub fn reroot_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    // Always root ourselves to the package root, and then compile relative to that.
    let rooted_path = SourcePackageLayout::try_find_root(&path.canonicalize()?)?;
    std::env::set_current_dir(rooted_path).unwrap();

    Ok(PathBuf::from("."))
}
