mod rpc;
mod block;
mod transaction;
mod blockchain;
mod keytool;
mod blockchain_simulation;
mod wallet;
mod gas;

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use serde_json::json;
use colored::Colorize;
use transaction::Transaction;
use std::fs;
use dirs;
use consensus_core::{NetworkConfig, NetworkType};
use p2p_protocol::P2PNetwork;
use crate::blockchain::{BALANCES, load_blockchain, save_blockchain};
use crate::blockchain_simulation::run_blockchain;
use crate::keytool::handle_keytool_command;
use crate::rpc::start_rpc_server;
use crate::wallet::print_coin_icon;

static CHAIN_ID: &str = "kari-c1";
static VERSION: &str = "0.3.0";

fn save_chain_id(chain_id: &str) -> io::Result<()> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;

    let config_file_path = config_path.join("config.json");
    let chain_id_json = json!({ "chain_id": chain_id });
    fs::write(config_file_path, serde_json::to_string_pretty(&chain_id_json)?)?;
    Ok(())
}

#[tokio::main]
async fn main() {


    let transaction = Transaction { sender: "Alice".to_string(), receiver: "Bob".to_string(), amount: 10, gas_cost: 0.1 };
    let total_cost = transaction.calculate_total_cost();
    println!("Total cost: {}", total_cost);
    

    let config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        port: 8080,
        peers: vec![],
        chain_id: "kari-c1".to_string(),
        max_connections: 100,
        api_enabled: true,
        network_type: NetworkType::Mainnet,
    };

    save_chain_id(&config.chain_id).expect("Failed to save chain id");

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