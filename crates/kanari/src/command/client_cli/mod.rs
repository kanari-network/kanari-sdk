// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Subcommand;

pub mod account;
pub mod balance;
pub mod burn;
pub mod envs;
pub mod faucet;
pub mod objects;
pub mod stats;
pub mod token_transfer;
pub mod transfer;
pub mod view;

#[derive(Subcommand, Debug)]
pub enum ClientCommand {
    /// Transfer Kanari tokens to another address
    Transfer(transfer::Transfer),
    /// Transfer custom tokens (e.g., JAMES) to another address
    TokenTransfer(token_transfer::TokenTransfer),
    /// Request tokens from the Dev faucet
    Faucet(faucet::Faucet),
    /// Burn Kanari tokens from a wallet (remove from total supply)
    Burn(burn::Burn),
    /// Show blockchain statistics
    Stats(stats::Stats),
    /// Show token balance for an address
    Balance(balance::Balance),
    /// Account operations (get account info)
    Account {
        #[command(subcommand)]
        command: account::AccountCommand,
    },
    /// Manage environments (RPC endpoints)
    Envs(envs::Envs),
    /// List owned objects
    Objects(objects::Objects),
    /// Call view functions (read-only, no transaction)
    View(view::View),
}

impl ClientCommand {
    pub async fn execute(&self) -> Result<()> {
        match self {
            ClientCommand::Transfer(cmd) => cmd.execute().await,
            ClientCommand::TokenTransfer(cmd) => cmd.execute().await,
            ClientCommand::Faucet(cmd) => cmd.execute().await,
            ClientCommand::Burn(cmd) => cmd.execute().await,
            ClientCommand::Stats(cmd) => cmd.execute().await,
            ClientCommand::Balance(cmd) => cmd.execute().await,
            ClientCommand::Account { command } => command.execute().await,
            ClientCommand::Envs(cmd) => cmd.execute(),
            ClientCommand::Objects(cmd) => cmd.execute().await,
            ClientCommand::View(cmd) => cmd.execute().await,
        }
    }
}
