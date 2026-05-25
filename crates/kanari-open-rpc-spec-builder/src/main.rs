// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "kanari-open-rpc-spec-builder")]
#[command(about = "Build and manage the Kanari OpenRPC specification")]
struct Cli {
    #[arg(value_enum, default_value_t = CliAction::Print)]
    action: CliAction,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliAction {
    Print,
    Test,
    Record,
}

impl From<CliAction> for kanari_open_rpc_spec_builder::Action {
    fn from(value: CliAction) -> Self {
        match value {
            CliAction::Print => Self::Print,
            CliAction::Test => Self::Test,
            CliAction::Record => Self::Record,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    kanari_open_rpc_spec_builder::run_action(cli.action.into())
}
