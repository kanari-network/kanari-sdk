mod blockchain_simulation;
mod config;
mod rpc;


use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::process::exit;
use colored::Colorize;
use command::keytool_cli::handle_keytool_command;
use command::move_cli::handle_move_command;
use config::{configure_network, load_config, save_config};
use key::{check_wallet_exists, list_wallet_files};
use network::{NetworkConfig, NetworkType};

use p2p_protocol::P2PNetwork;
use rpc::start_rpc_server;
use serde_json::json;
use k2::blockchain::{get_kari_dir, load_blockchain, save_blockchain, BALANCES};
use k2::chain_id::CHAIN_ID;
use crate::blockchain_simulation::run_blockchain;


static VERSION: &str = "0.2.2";

#[tokio::main]
async fn main() {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 1 {
        let command = &args[1];

        match command.as_str() {
            "start" => {
                start_node().await;
            },
            "move" => {
                handle_move_command();
            },
            "keytool" => {
                handle_keytool_command();
            },
            "version" | "--V" => {
                println!("CLI Version: {}", VERSION);
            },
            _ => {
                println!("{}", "Invalid command".red());
                println!("Usage: kari <command> [options]");
                println!("Commands:");
                println!("  {} - Start a local network", "start".green());
                println!("  {} - MoveVM", "move".green());
                println!("  {} - kari keystore tool", "keytool".green());
                println!("  {} - Show version", "version, --V".blue());
                exit(1);
            }
        }
    } else {
        // No command provided, print usage
        println!("Usage: kari <command> [options]");
        println!("Commands:");
        println!("  {} - Start a local network", "start".green());
        println!("  {} - MoveVM", "move".green());
        println!("  {} - kari keystore tool", "keytool".green());
        println!("  {} - Show version", "version, --V".blue());
        exit(1);
    }
}

async fn start_node() {

        // Check if any wallet exists first
        if !check_wallet_exists() {
            println!("{}", "No wallet found!".red());
            println!("Please create a wallet first using:");
            println!("{}", "kari keytool create".green());
            println!("Or import existing wallet using:");
            println!("{}", "kari keytool import".green());
            exit(1);
        }

    let mut config = load_config().expect("Failed to load configuration file");

    let _chain_id = config.get("chain_id").and_then(|v| v.as_str()).unwrap_or(CHAIN_ID);

    // Check if the configuration already exists
    let network_config = if config.get("network_type").is_some() && config.get("rpc_port").is_some() && config.get("domain").is_some() && config.get("chain_id").is_some() {
        println!("Configuration already exists. Skipping configuration process.");
        let network_type = match config.get("network_type").unwrap().as_str().unwrap() {
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            "mainnet" => NetworkType::Mainnet,
            _ => unreachable!(),
        };
        let rpc_port = config.get("rpc_port").unwrap().as_u64().unwrap() as u16;
        let _domain = config.get("domain").unwrap().as_str().unwrap().to_string();
        let chain_id = config.get("chain_id").unwrap().as_str().unwrap().to_string();

        NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            port: rpc_port,
            peers: vec![],
            chain_id,
            max_connections: 100,
            api_enabled: true,
            network_type,
        }
    } else {
        // Call configure_network and get the NetworkConfig
        match configure_network(CHAIN_ID) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error configuring network: {}", err);
                exit(1);
            }
        }
    };

    save_config(&json!({
        "chain_id": network_config.chain_id,
        "network_type": network_config.network_type.to_string(),
        "rpc_port": network_config.port,
        "domain": "kari.network"
    })).expect("Failed to save configuration");

    load_blockchain();
    let running = Arc::new(Mutex::new(true));

    unsafe {
        BALANCES = Some(Mutex::new(HashMap::new()));
    }


    // Load miner address with validation
    let miner_address = match config.get("miner_address").and_then(|v| v.as_str()) {
        Some(address) => {
            // Verify wallet file exists for this address
            if !std::path::Path::new(&get_kari_dir().join("wallets").join(format!("{}.json", address))).exists() {
                // Try to find any existing wallet
                match list_wallet_files() {
                    Ok(wallets) if !wallets.is_empty() => {
                        // Access first element of tuple (filename)
                        let first_wallet = wallets[0].0.trim_end_matches(".json").to_string();
                        println!("Using existing wallet as miner address: {}", first_wallet.green());
                        
                        // Update config with new miner address
                        let config = config.as_object_mut().unwrap();
                        config.insert("miner_address".to_string(), json!(first_wallet.clone()));
                        save_config(&json!(config)).expect("Failed to save configuration");
                        
                        first_wallet
                    },
                    _ => {
                        println!("{}", "No valid wallets found!".red());
                        println!("Please create a wallet first using:");
                        println!("{}", "kari keytool create".green());
                        exit(1);
                    }
                }
            } else {
                address.to_string()
            }
        },
        None => {
            // Try to find any existing wallet
            match list_wallet_files() {
                Ok(wallets) if !wallets.is_empty() => {
                    let first_wallet = wallets[0].0.trim_end_matches(".json").to_string();
                    println!("Setting miner address to existing wallet: {}", first_wallet.green());
                    
                    // Update config with new miner address
                    let config = config.as_object_mut().unwrap();
                    config.insert("miner_address".to_string(), json!(first_wallet.clone()));
                    save_config(&json!(config)).expect("Failed to save configuration");
                    
                    first_wallet
                },
                _ => {
                    println!("{}", "No wallets found!".red());
                    println!("Please create a wallet first using:");
                    println!("{}", "kari keytool create".green());
                    exit(1);
                }
            }
        }
    };

    loop {
        if miner_address.is_empty() {
            println!("Please generate an address first using the 'kari keytool' command.");
            break; 
        } else {
            println!("Using existing miner address: {}", miner_address);
            *running.lock().unwrap() = true;
            println!("{}", "Starting blockchain...".green());
            let running_clone = Arc::clone(&running);
            let miner_address_clone = miner_address.clone();

            let p2p_network = P2PNetwork::new();

            tokio::spawn(async move {
                println!("Starting P2P network listener...");
                if let Err(e) = p2p_network.start_listener("127.0.0.1:8080").await {
                    println!("Failed to start P2P network listener: {}", e);
                }
            });

     
            tokio::spawn(async move {
                println!("Starting RPC server...");
                start_rpc_server(network_config).await;
            });

            tokio::spawn(async move {
                println!("Running blockchain simulation...");
                run_blockchain(running_clone, miner_address_clone);
            });
        }

        let mut input = String::new();
        println!("{} to stop the node", "Press Enter".yellow());
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        *running.lock().unwrap() = false;
        println!("{}", "Stopping blockchain...".red());
        save_blockchain();
        break;
    }
}