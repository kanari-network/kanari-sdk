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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde_json::{json, Value};
use colored::Colorize;
use std::fs::{self, File};
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

// Function to get the configuration directory
fn get_config_dir() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;
    Ok(config_path)
}

// Function to load the configuration from file
fn load_config() -> io::Result<Value> {
    let config_file_path = get_config_dir()?.join("config.json");
    if !config_file_path.exists() {
        return Ok(json!({})); // Return an empty JSON object if the file doesn't exist
    }
    let config_str = fs::read_to_string(config_file_path)?;
    Ok(serde_json::from_str(&config_str)?)
}

// Function to save the configuration to file
fn save_config(config: &Value) -> Result<(), std::io::Error> {
    let config_dir = get_config_dir()?;
    let config_file_path = config_dir.join("config.json");
    let mut file = File::create(config_file_path)?;
    file.write_all(config.to_string().as_bytes())?;
    Ok(())
}

// Function to prompt the user for a value with a default
fn prompt_for_value(prompt: &str, default: &str) -> String {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        default.to_string()
    } else {
        trimmed_input.to_string()
    }
}

// Function to configure the network settings
pub fn configure_network() -> io::Result<NetworkConfig> {
    let mut config = load_config()?;

    println!("Choose a network type:");
    println!("1. devnet");
    println!("2. testnet");
    println!("3. mainnet");

    let network_type_input = loop {
        print!("Enter your choice [1-3]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().parse::<u32>() {
            Ok(1) => break "devnet",
            Ok(2) => break "testnet",
            Ok(3) => break "mainnet",
            _ => println!("Invalid choice. Please enter a number between 1 and 3."),
        }
    };

    let network_type = match network_type_input {
        "devnet" => NetworkType::Devnet,
        "testnet" => NetworkType::Testnet,
        "mainnet" => NetworkType::Mainnet,
        _ => unreachable!(), // We already validated the input
    };

    config.as_object_mut().unwrap().insert("network_type".to_string(), json!(network_type_input));

    let default_rpc_port = match network_type_input {
        "devnet" => "3031",
        "testnet" => "3032",
        "mainnet" => "3030",
        _ => "3030", // Default to mainnet port
    };
    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    config.as_object_mut().unwrap().insert("rpc_port".to_string(), json!(rpc_port));

    let default_domain = match network_type_input {
        "devnet" => "devnet.kari.network",
        "testnet" => "testnet.kari.network",
        "mainnet" => "kari.network",
        _ => "kari.network", // Default to mainnet domain
    };
    let domain = prompt_for_value("Enter network domain", default_domain);
    config.as_object_mut().unwrap().insert("domain".to_string(), json!(domain));

    // Save the chain ID to the configuration
    config.as_object_mut().unwrap().insert("chain_id".to_string(), json!(CHAIN_ID));

    let network_config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        port: rpc_port, // Use the parsed rpc_port
        peers: vec![],
        chain_id: CHAIN_ID.to_string(),
        max_connections: 100,
        api_enabled: true,
        network_type,
    };

    save_config(&config)?;

    println!("Network configuration saved successfully.");
    Ok(network_config) // Return the NetworkConfig
}

#[tokio::main]
async fn main() {
    let config = load_config().expect("Failed to load configuration");

    let _chain_id = config.get("chain_id").and_then(|v| v.as_str()).unwrap_or(CHAIN_ID);

    // Call configure_network and get the NetworkConfig
    let network_config = match configure_network() {
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