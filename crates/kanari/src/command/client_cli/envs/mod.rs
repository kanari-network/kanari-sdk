// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kanari_common::{add_env, get_active_env, get_envs, remove_env, set_active_env};

/// Manage environments (RPC endpoints)
#[derive(Parser, Debug)]
pub struct Envs {
    #[command(subcommand)]
    pub command: EnvsCommand,
}

#[derive(Subcommand, Debug)]
pub enum EnvsCommand {
    /// List all configured environments
    List,
    /// Switch to a different environment
    Switch {
        /// Alias of the environment to switch to
        alias: String,
    },
    /// Add a new environment
    New {
        /// Alias for the environment (e.g., localnet, devnet)
        alias: String,
        /// RPC endpoint URL
        rpc: String,
    },
    /// Remove an environment
    Remove {
        /// Alias of the environment to remove
        alias: String,
    },
}

impl Envs {
    pub fn execute(&self) -> Result<()> {
        match &self.command {
            EnvsCommand::List => {
                let envs = get_envs().unwrap_or_default();
                let active = get_active_env();

                eprintln!("{:<15} {:<30}", "ALIAS", "RPC ENDPOINT");
                eprintln!("{:-<15} {:-<30}", "", "");

                for (alias, rpc) in envs {
                    let mark = if active.as_deref() == Some(&alias) {
                        "*"
                    } else {
                        " "
                    };
                    eprintln!("{:<15} {:<30} {}", alias, rpc, mark);
                }

                if let Some(a) = active {
                    eprintln!("\n* Active environment: {}", a);
                } else {
                    eprintln!(
                        "\nNo active environment set. Use `kanari client envs switch <alias>` to set one."
                    );
                }
                Ok(())
            }
            EnvsCommand::Switch { alias } => {
                set_active_env(alias)
                    .with_context(|| format!("Failed to switch to environment '{}'", alias))?;
                eprintln!("Switched to environment '{}'", alias);
                Ok(())
            }
            EnvsCommand::New { alias, rpc } => {
                add_env(alias, rpc)
                    .with_context(|| format!("Failed to add environment '{}'", alias))?;
                eprintln!("Added environment '{}' with RPC '{}'", alias, rpc);
                Ok(())
            }
            EnvsCommand::Remove { alias } => {
                remove_env(alias)
                    .with_context(|| format!("Failed to remove environment '{}'", alias))?;
                eprintln!("Removed environment '{}'", alias);
                Ok(())
            }
        }
    }
}
