// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Main CLI binary for Kanari - A Move-based money transfer system



use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

pub mod command;
use command::client_cli;
use command::keytool_cli;
use command::move_cli;

/// Kanari - A Move-based money transfer system
#[derive(Parser)]
#[command(name = "kanari")]
#[command(about = "Money transfer system using Move language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Client operations (transfer, faucet, burn, stats, balance, account)
    Client {
        #[command(subcommand)]
        command: client_cli::ClientCommand,
    },
    /// Manage Move packages and tools
    Move {
        #[command(subcommand)]
        command: move_cli::MoveCommand,
    },
    /// Keytool operations (create, load, list, info, add wallets)
    Keytool {
        #[command(subcommand)]
        command: keytool_cli::KeytoolCommand,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Use tokio runtime for async RPC calls
    let runtime = tokio::runtime::Runtime::new()?;

    match cli.command {
        Commands::Client { command } => {
            runtime.block_on(async { command.execute().await })?;
            Ok(())
        }

        Commands::Keytool { command } => {
            command.execute()?;
            Ok(())
        }

        Commands::Move { command } => {
            // Dispatch into the move CLI helper
            command
                .execute()
                .context("Failed to execute move subcommand")?;

            Ok(())
        }
    }
}
