mod rpc;
mod block;
mod transaction;
mod blockchain;
mod keytool;
mod blockchain_simulation;
mod wallet;
mod gas;
mod config;

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use colored::Colorize;
use config::{configure_network, load_config, save_config};
use p2p_protocol::P2PNetwork;
use serde_json::json;
use crate::blockchain::{BALANCES, load_blockchain, save_blockchain};
use crate::blockchain_simulation::run_blockchain;
use crate::keytool::handle_keytool_command;
use crate::rpc::start_rpc_server;
use crate::wallet::print_coin_icon;


static CHAIN_ID: &str = "kari-c1";
static VERSION: &str = "0.3.0";

fn main() {
    let config = load_config().expect("Failed to load configuration");

    let _chain_id = config.get("chain_id").and_then(|v| v.as_str()).unwrap_or(CHAIN_ID);

    // Call configure_network and get the NetworkConfig
    let network_config = match configure_network(CHAIN_ID) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error configuring network: {}", err);
            return;
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

    let mut miner_address = String::new();
    loop {
        let mut input = String::new(); // Reset input at the beginning of each loop iteration
        println!("\nAvailable Commands:");
        println!("{} - Start a local network", "start".green());
        println!("{} - kari keystore tool", "keytool".green());
        println!("{} - move", "move".green());
        println!("{} - Stop the blockchain and exit", "stop".red());
        println!("{} - Show version", "version, --V".blue());

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "start" => {
                if miner_address.is_empty() {
                    println!("Please generate an address first using the keytool command.");
                } else {
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
                        start_rpc_server().await;
                    });

                    tokio::spawn(async move {
                        println!("Running blockchain simulation...");
                        run_blockchain(running_clone, miner_address_clone);
                    });
                }
            },
            "keytool" => {
                let result = handle_keytool_command();
                if let Some(address) = result {
                    miner_address = address;
                }
            },
            "move" => {
                
            },
            "stop" => {
                *running.lock().unwrap() = false;
                println!("{}", "Stopping blockchain...".red());
                save_blockchain();
                break;
            },
            "version" => {
                println!("CLI Version: {}", VERSION);
            },
            "--V" => {
                println!("CLI Version: {}", VERSION);
            },
            _ => println!("{}", "Invalid command".red()),
        }
    }
}