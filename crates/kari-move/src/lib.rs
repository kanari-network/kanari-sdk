// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_package::BuildConfig;

pub mod base;
pub mod sandbox;

/// Default directory where saved Move resources live
pub const DEFAULT_STORAGE_DIR: &str = "storage";

/// Default directory for build output
pub const DEFAULT_BUILD_DIR: &str = ".";

use anyhow::Result;
use clap::Parser;
use move_core_types::{
    account_address::AccountAddress, errmap::ErrorMapping, identifier::Identifier,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_test_utils::gas_schedule::CostTable;
use std::path::PathBuf;

type NativeFunctionRecord = (AccountAddress, Identifier, Identifier, NativeFunction);

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Move {
    /// Path to a package which the command should be run with respect to.
    #[clap(long = "path", short = 'p', global = true)]
    pub package_path: Option<PathBuf>,

    /// Print additional diagnostics if available.
    #[clap(short = 'v', global = true)]
    pub verbose: bool,

    /// Package build options
    #[clap(flatten)]
    pub build_config: BuildConfig,
}

/// MoveCLI is the CLI that will be executed by the `move-cli` command
/// The `cmd` argument is added here rather than in `Move` to make it
/// easier for other crates to extend `move-cli`
#[derive(Parser)]
pub struct MoveCLI {
    #[clap(flatten)]
    pub move_args: Move,

    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Parser)]
pub enum Command {
    Build(base::Build),
    Coverage(base::Coverage),
    Disassemble(base::Disassemble),
    Docgen(base::Docgen),
    Errmap(base::Errmap),
    Info(base::Info),
    Migrate(base::Migrate),
    New(base::New),
    Test(base::Test),
    Sandbox {
        /// Directory storing Move resources, events, and module bytecodes produced by module publishing
        /// and script execution.
        #[clap(long, default_value = DEFAULT_STORAGE_DIR)]
        storage_dir: PathBuf,
        #[clap(subcommand)]
        cmd: sandbox::cli::SandboxCommand,
    },
}

pub fn run_cli(
    files: Vec<PathBuf>,
    cost_table: &CostTable,
    error_descriptions: &ErrorMapping,
    args: Move,
    command: Command,
) -> Result<()> {
    let _ = files;
    match command {
        Command::Build(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Coverage(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Disassemble(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Docgen(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Errmap(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Info(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::Migrate(cmd) => cmd.execute(args.package_path, args.build_config),
        Command::New(cmd) => cmd.execute_with_defaults(args.package_path),
        Command::Test(cmd) => cmd.execute(
            args.package_path,
            args.build_config,
            vec![],
            Some(cost_table.clone()),
        ),
        Command::Sandbox { storage_dir, cmd } => {
            cmd.handle_command(vec![], cost_table, error_descriptions, &args, &storage_dir)
        }
    }
}

pub fn move_cli(
    natives: Vec<NativeFunctionRecord>,
    cost_table: &CostTable,
    error_descriptions: &ErrorMapping,
) -> Result<()> {
    let _ = natives;
    let args = MoveCLI::parse();
    run_cli(
        vec![],
        cost_table,
        error_descriptions,
        args.move_args,
        args.cmd,
    )
}
