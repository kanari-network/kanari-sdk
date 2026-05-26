// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Main entry point for Kanari blockchain node
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use kanari_crypto::wallet::list_wallet_files;

mod app;
mod indexer;
mod p2p;
mod peer_store;
mod sync;
use app::{create_engine, default_data_dir, print_account, print_block, print_stats, run_node};

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum NetworkMode {
    Mainnet,
    Testnet,
    Devnet,
}

impl NetworkMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Devnet => "devnet",
        }
    }
}

/// Kanari node command-line interface
#[derive(Parser)]
#[command(name = "kanari-node", about = "Kanari run server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// start the node
    Start {
        /// Network mode for runtime and production safety defaults
        #[arg(long, value_enum, default_value = "testnet")]
        network: NetworkMode,
        /// P2P listen port
        #[arg(long, default_value = "19000")]
        p2p_port: u16,
        /// RPC listen port
        #[arg(long, default_value = "19001")]
        rpc_port: u16,
        /// RPC listen host/IP (use 0.0.0.0 to bind all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        rpc_host: String,
        /// Data directory for blockchain and state storage
        #[arg(long)]
        data_dir: Option<std::path::PathBuf>,
        /// Run as relay server to help other nodes behind NAT
        #[arg(long, default_value = "false")]
        relay_server: bool,
        /// Authority ID for DAG consensus (e.g. 0x1)
        #[arg(long)]
        authority_id: Option<String>,
        /// List of authority IDs for DAG consensus (comma-separated)
        #[arg(long, value_delimiter = ',')]
        authorities: Option<Vec<String>>,
        /// Bootstrap peer multiaddr to connect to (can be specified multiple times)
        #[arg(long, value_name = "MULTIADDR")]
        bootstrap: Option<Vec<String>>,
    },
    /// Run a local-only node
    Local,
    /// List wallet files
    ListWallets,
    /// Show blockchain statistics
    Stats,
    /// Get account info
    Account { address: String },
    /// Get block information by height
    Block { height: u64 },
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn validate_start_authority_config(
    authority_id: &Option<String>,
    authorities: &Option<Vec<String>>,
) -> Result<()> {
    match (authority_id.as_ref(), authorities.as_ref()) {
        (Some(_), Some(authorities)) if !authorities.is_empty() => Ok(()),
        (Some(_), Some(_)) => {
            anyhow::bail!("DAG multi-node start requires a non-empty --authorities list")
        }
        (None, None) => anyhow::bail!(
            "kanari-node start requires --authority-id and --authorities for deterministic multi-node DAG startup; use the Local command for local-only mode"
        ),
        _ => anyhow::bail!(
            "kanari-node start requires both --authority-id and --authorities together"
        ),
    }
}

fn main() -> Result<()> {
    // Initialize tracing subscriber first so all commands have log output
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::ListWallets => {
            for (addr, selected) in list_wallet_files()? {
                tracing::info!("{}{}", addr, if selected { " (selected)" } else { "" });
            }
            Ok(())
        }
        Commands::Stats => print_stats(),
        Commands::Account { address } => print_account(&address),
        Commands::Block { height } => print_block(height),
        Commands::Start {
            network,
            p2p_port,
            rpc_port,
            rpc_host,
            data_dir,
            relay_server,
            authority_id,
            authorities,
            bootstrap,
        } => {
            validate_start_authority_config(&authority_id, &authorities)?;
            let data_dir_path = data_dir.clone().unwrap_or_else(default_data_dir);
            let mut engine = create_engine(&data_dir, &network)?;

            let id = authority_id.expect("validated authority_id must exist");
            let auths = authorities.expect("validated authorities must exist");
            tracing::info!(
                "Configuring Authority ID: {} with {} authorities",
                id,
                auths.len()
            );
            engine.set_authorities(id, auths);

            runtime()?.block_on(run_node(
                std::sync::Arc::new(engine),
                network.as_str().to_string(),
                p2p_port,
                rpc_port,
                rpc_host,
                data_dir_path,
                relay_server,
                bootstrap,
            ))
        }
        Commands::Local => {
            tracing::info!("Starting local node: RPC on 127.0.0.1:6767 (P2P disabled)");
            let data_dir_path = std::path::PathBuf::from("./.kanari-local");
            // Ensure data directory exists
            std::fs::create_dir_all(&data_dir_path)?;
            let data_dir = Some(data_dir_path.clone());
            let engine = create_engine(&data_dir, &NetworkMode::Devnet)?;
            runtime()?.block_on(run_node(
                std::sync::Arc::new(engine),
                NetworkMode::Devnet.as_str().to_string(),
                0,
                6767,
                "127.0.0.1".to_string(),
                data_dir_path,
                false,
                None,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_start_authority_config;

    #[test]
    fn start_requires_both_authority_fields() {
        assert!(validate_start_authority_config(&None, &None).is_err());
        assert!(validate_start_authority_config(&Some("0x1".to_string()), &None).is_err());
        assert!(validate_start_authority_config(&None, &Some(vec!["0x1".to_string()])).is_err());
    }

    #[test]
    fn start_rejects_empty_authority_list() {
        assert!(validate_start_authority_config(&Some("0x1".to_string()), &Some(vec![])).is_err());
    }

    #[test]
    fn start_accepts_complete_authority_config() {
        assert!(
            validate_start_authority_config(
                &Some("0x1".to_string()),
                &Some(vec!["0x1".to_string(), "0x2".to_string()]),
            )
            .is_ok()
        );
    }
}
