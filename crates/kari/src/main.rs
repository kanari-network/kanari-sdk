
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::process::exit;
use colored::Colorize;
use command::keytool_cli::handle_keytool_command;
use command::move_cli::handle_move_command;

use k2::simulation::run_blockchain;
use key::{check_wallet_exists, list_wallet_files};
use network::{NetworkConfig, NetworkType};

use p2p_protocol::P2PNetwork;

use serde_json::json;
use k2::blockchain::{get_kari_dir, load_blockchain, save_blockchain, BALANCES};
use k2::chain_id::CHAIN_ID;
use k2::config::{configure_network, load_config, save_config};
use k2::rpc::start_rpc_server;


static VERSION: &str = env!("CARGO_PKG_VERSION");


struct CommandInfo {
    name: &'static str,
    alias: Option<&'static str>,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo { name: "start", alias: None, description: "Start a local network" },
    CommandInfo { name: "move", alias: None, description: "MoveVM" },
    CommandInfo { name: "keytool", alias: None, description: "Kari keystore tool" },
    CommandInfo { name: "version", alias: Some("--V"), description: "Show version" },
];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  kari <command> [options]\n");

    // Commands
    println!("{}", "COMMANDS:".bright_yellow().bold());
    
    let max_name_len = COMMANDS.iter()
        .map(|cmd| cmd.name.len() + cmd.alias.map_or(0, |a| a.len() + 2))
        .max()
        .unwrap_or(0);
    
    for cmd in COMMANDS {
        let name = match cmd.alias {
            Some(alias) => format!("{}, {}", cmd.name, alias),
            None => cmd.name.to_string()
        };
        
        println!(
            "  {}{}  {}", 
            name.green().bold(),
            " ".repeat(max_name_len - name.len() + 2),
            cmd.description.bright_white()
        );
    }
    println!();
    
    exit(1);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() <= 1 {
        display_help(false);
    }

    match args.get(1).map(|s| s.as_str()) {
        Some("start") => start_node().await,
        Some("move") => handle_move_command(),
        Some("keytool") => {
            let _ = handle_keytool_command(); // Ignore Option<String> return value
        },
        Some("version") | Some("--V") => println!("CLI Version: {}", VERSION),
        _ => display_help(true),
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
            if !std::path::Path::new(&get_kari_dir().join("wallets").join(format!("{}.toml", address))).exists() {
                // Try to find any existing wallet
                match list_wallet_files() {
                    Ok(wallets) if !wallets.is_empty() => {
                        // Access first element of tuple (filename)
                        let first_wallet = wallets[0].0.trim_end_matches(".toml").to_string();
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
                    let first_wallet = wallets[0].0.trim_end_matches(".toml").to_string();
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