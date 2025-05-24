use network::NetworkConfig;
use panorama::simulation::run_blockchain;

use common::{get_kari_dir, load_kanari_config, save_kanari_config};
use mona_blockchain::blockchain::{load_blockchain, save_blockchain};
use mona_blockchain::chain_id::CHAIN_ID;
use panorama::config::{configure_network, load_config, save_config};
use rpc_api::start_rpc_server;


use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

use mona_crypto::{check_wallet_exists, list_wallet_files};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use std::process::exit;

use colored::Colorize;
use serde_yaml::Value;

pub mod generate_certs;

struct CommandInfo {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "start",
        description: "Start the Kari server",
    },
    CommandInfo {
        name: "generate-certs",
        description: "Generate SSL certificates",
    },
];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage section
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("kari server <command> [options]\n");

    // Commands section
    println!("{}", "COMMANDS:".bright_yellow().bold());

    let max_name_len = COMMANDS.iter().map(|cmd| cmd.name.len()).max().unwrap_or(0);

    for cmd in COMMANDS {
        println!(
            "  {}{}  {}",
            cmd.name.green().bold(),
            " ".repeat(max_name_len - cmd.name.len() + 2),
            cmd.description.bright_white()
        );
    }
    println!();

    exit(1);
}

// Handle server commands
pub async fn handle_server_command() -> Option<String> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check if any arguments were provided
    if args.len() > 2 {
        // Collect command line arguments
        let command = &args[2];
        // Use string comparison in the match statement
        match command.as_str() {

            "start" => {
                // Extract peer and port arguments
                let mut peers = Vec::new();
                let mut port = None;
                let mut localhost_only = false;

                let mut i = 2;
                while i < args.len() {
                    match args.get(i).map(|s| s.as_str()) {
                        Some("--peer") => {
                            if let Some(peer_addr) = args.get(i + 1) {
                                peers.push(peer_addr.to_string());
                                i += 2;
                            } else {
                                eprintln!("Error: --peer requires an address argument");
                                exit(1);
                            }
                        }
                        Some("--port") => {
                            if let Some(port_str) = args.get(i + 1) {
                                match port_str.parse::<u16>() {
                                    Ok(p) => {
                                        port = Some(p);
                                        i += 2;
                                    }
                                    Err(_) => {
                                        eprintln!("Error: Invalid port number");
                                        exit(1);
                                    }
                                }
                            } else {
                                eprintln!("Error: --port requires a number argument");
                                exit(1);
                            }
                        }
                        Some("--localhost") => {
                            localhost_only = true;
                            i += 1;
                        }
                        _ => {
                            eprintln!("Unknown argument: {}", args[i]);
                            display_help(true);
                            i += 1;
                        }
                    }
                }

                start_server(peers, port, localhost_only).await;
                Some("Server started successfully".to_string())
            }

            "generate-certs" => {
                match generate_certs::generate_ssl_certificates() {
                    Ok(_) => Some("SSL certificates generated successfully".to_string()),
                    Err(e) => {
                        eprintln!("{}", format!("Error generating certificates: {}", e).red().bold());
                        None
                    }
                }
            }

            _ => {
                display_help(true);
                None
            }
        }
    } else {
        display_help(false);
        return None;
    }
}

async fn start_server(peers: Vec<String>, port: Option<u16>, localhost_only: bool) {
    println!("{}", "Starting Kari server...".bright_green().bold());
    
    // Load or create Kanari configuration
    let kari_dir = get_kari_dir();
    let config_path = kari_dir.join("config.yaml");
    let mut kanari_config = load_kanari_config().unwrap_or_default();
    
    // Set up blockchain directory
    let blockchain_dir = kari_dir.join("blockchain");
    if !blockchain_dir.exists() {
        std::fs::create_dir_all(&blockchain_dir).expect("Failed to create blockchain directory");
    }
    
    // Check for wallet
    let wallet_files = list_wallet_files().unwrap_or_default();
    let has_wallet = !wallet_files.is_empty();
    
    if !has_wallet {
        println!("{}", "No wallet found. Creating a new wallet...".yellow().bold());
        
        // Here you would typically prompt for a password and create a wallet
        // For now, we'll just exit with an error
        println!("{}", "Please create a wallet first using 'kari wallet create'".red().bold());
        exit(1);
    }
    
    // Load the first wallet for blockchain identity
    // Extract the wallet address string from the tuple (wallet_address, is_default)
    let (default_wallet, _) = wallet_files.first().unwrap().clone();
    println!("Using wallet: {}", default_wallet.bright_white());
    
    // Load existing blockchain or create new one
    let blockchain = load_blockchain().unwrap_or_default();
    
    // Set up network configuration
    let rpc_port = port.unwrap_or(30030); // Default RPC port
    let peers_count = peers.len();
    let mut network_config = NetworkConfig {
        peers,
        port: rpc_port,
        chain_id: CHAIN_ID.to_string(),
        localhost_only,
        ..Default::default()
    };
    
    // Load node configuration from existing config if available
    // Properly handle the Result returned by load_config()
    if let Ok(panorama_config) = load_config() {
        // Use the chain_id from the panorama_config to configure the network
        if let Ok(config) = configure_network(&CHAIN_ID.to_string()) {
            // Merge relevant config items but keep our overrides
            network_config.peers.extend(config.peers);
            // Keep our port, chain_id, and localhost_only settings
        }
    }
    
    // Create communication channels
    let (tx, rx) = mpsc::channel::<String>(100);
    let running = Arc::new(Mutex::new(true));
    
    // Clone for thread use
    let running_clone = running.clone();
    
    // Start blockchain in separate thread
    let blockchain_handle = tokio::spawn(async move {
        // Start blockchain simulation with the wallet address
        run_blockchain(running_clone, default_wallet, tx);
    });
    
    // Start RPC server with the correct parameters
    println!("Starting RPC server on port {}", rpc_port);
    let rpc_handle = tokio::spawn(async move {
        // Configure and start the RPC server with network_config
        let _ = start_rpc_server(network_config).await;
    });
    
    // Update Kanari config with latest settings
    // Handle serde_yaml::Value properly
    if let Some(mapping) = kanari_config.as_mapping_mut() {
        mapping.insert(Value::from("rpc_port"), Value::from(rpc_port));
        mapping.insert(Value::from("localhost_only"), Value::from(localhost_only));
    }
    let _ = save_kanari_config(&kanari_config);
    
    // Print server info
    println!("\n{}", "Kari Server is running!".bright_green().bold());
    println!("RPC endpoint: {}", format!("http://localhost:{}", rpc_port).bright_white());
    if localhost_only {
        println!("Mode: {}", "Localhost only".yellow());
    } else {
        println!("Mode: {}", "Network enabled".green());
    }
    println!("Connected peers: {}", peers_count.to_string().bright_white());
    println!("\nPress Ctrl+C to stop the server\n");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c event");
    
    println!("\n{}", "Shutting down Kari server...".yellow().bold());
    
    // Set running flag to false to stop blockchain
    {
        let mut running_flag = running.lock().unwrap();
        *running_flag = false;
    }
    
    // Wait for blockchain to stop (with timeout)
    let _ = tokio::time::timeout(Duration::from_secs(5), blockchain_handle).await;
    
    // Save blockchain state before exit
    let _ = save_blockchain();
    
    println!("{}", "Kari server stopped.".bright_green().bold());
}
