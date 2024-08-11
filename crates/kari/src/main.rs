mod api;
mod keytool;
mod blockchain_simulation;
mod config;
mod rpc;
mod movekari;


use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::process::exit;
use colored::Colorize;
use config::{configure_network, load_config, save_config};
use key::{generate_karix_address, print_coin_icon, save_wallet};
use network::{NetworkConfig, NetworkType};
use movekari::handle_move_command;
use p2p_protocol::P2PNetwork;
use serde_json::json;
use simulation::blockchain::{load_blockchain, save_blockchain, BALANCES};
use simulation::chain_id::CHAIN_ID;
use crate::blockchain_simulation::run_blockchain;
use crate::keytool::handle_keytool_command;
use crate::api::start_api_server;
use crate::rpc::start_rpc_server;


static VERSION: &str = "0.3.0";

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
    let mut config = load_config().expect("Failed to load configuration");

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

    println!("{}", "Welcome to the Rust Blockchain CLI".bold().cyan());
    print_coin_icon();

    // Load miner address from config if it exists, otherwise generate a new one
    let miner_address = match config.get("miner_address").and_then(|v| v.as_str()) {
        Some(address) => address.to_string(),
        None => {
            println!("No miner address found. Generating a new one...");
            let (private_key, public_address, seed_phrase) = generate_karix_address(24); // Use 24 words for security
            println!("New address generated:");
            println!("Private Key: {}", private_key.green());
            println!("Public Address: {}", public_address.green());
            println!("Seed Phrase: {}", seed_phrase.green());

            save_wallet(&public_address, &private_key, &seed_phrase);

            // Save the new address to the configuration
            config.as_object_mut().unwrap().insert("miner_address".to_string(), json!(public_address));
            save_config(&config).expect("Failed to save configuration");

            public_address
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
                println!("Starting API server...");
                start_api_server().await;
            });

            tokio::spawn(async move {
                println!("Running blockchain simulation...");
                run_blockchain(running_clone, miner_address_clone);
            });

            // Start RPC server
            let rpc_port = network_config.port;
            tokio::spawn(async move {
                println!("Starting RPC server...");
                start_rpc_server(rpc_port);
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