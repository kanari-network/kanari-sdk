use network::NetworkConfig;
use panorama::simulation::run_blockchain;

use common::{get_kari_dir, load_kanari_config, save_kanari_config, load_config, save_config};
use mona_blockchain::blockchain::{load_blockchain, save_blockchain};
use mona_blockchain::chain_id::CHAIN_ID;
use panorama::config::{configure_network,};
use rpc_api::start_rpc_server;


use tokio::sync::mpsc;
use tokio::time::Duration;

use mona_crypto::{check_wallet_exists, list_wallet_files};
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
    if args.len() <= 2 {
        // No subcommand provided
        println!("{}", "Server commands:".bright_green().bold());
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
        
        // Instead of exiting, just return None to go back to main
        return None;
    }

    // Collect command line arguments
    let command = &args[2];
    // Use string comparison in the match statement
    match command.as_str() {

        "start" => {
            // Extract peer and port arguments
            let mut peers = Vec::new();
            let mut port = None;
            let mut localhost_only = false;
            let mut selected_wallet = None;
            let mut use_tls = true; // Default to TLS enabled

            let mut i = 3; // Start at index 3 to skip program name, "server", and "start" command
            while i < args.len() {
                match args.get(i).map(|s| s.as_str()) {
                    Some("--peer") => {
                        if let Some(peer_addr) = args.get(i + 1) {
                            peers.push(peer_addr.to_string());
                            i += 2;
                        } else {
                            eprintln!("{}", "Error: --peer requires an address argument".red().bold());
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
                                    eprintln!("{}", "Error: Invalid port number".red().bold());
                                    exit(1);
                                }
                            }
                        } else {
                            eprintln!("{}", "Error: --port requires a number argument".red().bold());
                            exit(1);
                        }
                    }
                    Some("--localhost") => {
                        if let Some(value) = args.get(i + 1) {
                            // Better handling of boolean values with typo tolerance
                            localhost_only = match value.to_lowercase().as_str() {
                                "true" | "t" | "yes" | "y" | "1" | "ture" => true,
                                "false" | "f" | "no" | "n" | "0" => false,
                                _ => {
                                    println!("{}", format!("Warning: Invalid value for --localhost: '{}', defaulting to false", value).yellow().bold());
                                    false
                                },
                            };
                            i += 2;
                        } else {
                            // If no value provided, assume true (flag presence implies true)
                            localhost_only = true;
                            i += 1;
                        }
                    }
                    Some("--no-tls") => {
                        use_tls = false;
                        i += 1;
                    }
                    Some("--wallet") => {
                        if let Some(wallet_addr) = args.get(i + 1) {
                            selected_wallet = Some(wallet_addr.to_string());
                            i += 2;
                        } else {
                            eprintln!("{}", "Error: --wallet requires an address argument".red().bold());
                            exit(1);
                        }
                    }
                    Some(unknown_arg) => {
                        eprintln!("{}", format!("Unknown argument: {}", unknown_arg).red().bold());
                        display_help(true);
                        exit(1);
                    }
                    None => {
                        i += 1; // Skip any potential null arguments
                    }
                }
            }

            println!("{}", "Preparing to start server...".bright_green());
            if !peers.is_empty() {
                println!("Connecting to peers: {}", peers.join(", ").bright_white());
            }
            if let Some(p) = port {
                println!("Using custom port: {}", p.to_string().bright_white());
            }

            if let Err(err) = start_server(peers, port, localhost_only, selected_wallet, use_tls).await {
                eprintln!("{}", format!("Server startup failed: {}", err).red().bold());
                return None;
            }
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
}

async fn start_server(
    peers: Vec<String>, 
    port: Option<u16>, 
    localhost_only: bool,
    selected_wallet: Option<String>,
    use_tls: bool,
) -> Result<(), String> {
    println!("{}", "Starting Kari server...".bright_green().bold());
    
    // Load or create Kanari configuration
    let kari_dir = get_kari_dir();
    let _config_path = kari_dir.join("config.yaml");
    let mut kanari_config = match load_kanari_config() {
        Ok(config) => config,
        Err(e) => {
            println!("{}", format!("Warning: Failed to load config: {}", e).yellow().bold());
            Value::default()
        }
    };
    
    // Set up blockchain directory
    let blockchain_dir = kari_dir.join("blockchain");
    if !blockchain_dir.exists() {
        std::fs::create_dir_all(&blockchain_dir)
            .map_err(|e| format!("Failed to create blockchain directory: {}", e))?;
    }
    
    // Check for wallet using the direct function
    if !check_wallet_exists() {
        println!("{}", "No wallet found!".red().bold());
        println!("{}", "Please create a wallet first using 'kari keytool generate'".yellow().bold());
        return Err("No wallet found".to_string());
    }
    
    // Load available wallet files
    let wallet_files = list_wallet_files()
        .map_err(|e| format!("Failed to list wallet files: {}", e))?;
    
    if wallet_files.is_empty() {
        println!("{}", "No wallet files found. Please create a wallet first.".red().bold());
        return Err("No wallet files found".to_string());
    }
    
    // Select wallet based on arguments or default
    let default_wallet = if let Some(selected) = selected_wallet {
        // Find the selected wallet in available wallets
        let selected_exists = wallet_files.iter().any(|(addr, _)| addr == &selected);
        if !selected_exists {
            println!("{}", format!("Warning: Selected wallet {} not found, using default", selected).yellow().bold());
            wallet_files.first().unwrap().0.clone()
        } else {
            selected
        }
    } else {
        // Find default wallet (marked as default) or use first one
        match wallet_files.iter().find(|(_, is_default)| *is_default) {
            Some((addr, _)) => addr.clone(),
            None => wallet_files.first().unwrap().0.clone(),
        }
    };
    
    println!("Using wallet: {}", default_wallet.bright_white());
    
    // Load existing blockchain or create new one
    match load_blockchain() {
        Ok(_) => println!("Blockchain loaded successfully"),
        Err(e) => {
            println!("{}", format!("Warning: Failed to load blockchain: {}", e).yellow().bold());
            println!("Creating new blockchain...");
            // Continue anyway as a new blockchain will be created if needed
        }
    }
    
    // Set up network configuration
    let rpc_port = port.unwrap_or(30030); // Default RPC port
    let peers_count = peers.len();
    
    // Create more comprehensive network config
    let mut network_config = NetworkConfig {
        peers: peers.clone(),
        port: rpc_port,
        chain_id: CHAIN_ID.to_string(),
        localhost_only,
        node_address: "0.0.0.0".to_string(), // Listen on all interfaces
        max_connections: 100,                // Reasonable default
        api_enabled: true,                   // Enable API endpoints
        use_tls,                            // Default to no TLS
        trusted_peers: Vec::new(),           // No trusted peers by default
    };
    
    // Load node configuration from existing config if available
    if let Ok(_panorama_config) = load_config() {
        println!("Loaded existing node configuration");
        
        // Use the chain_id from the panorama_config to configure the network
        if let Ok(config) = configure_network(&CHAIN_ID.to_string()) {
            println!("Merging peer configurations...");
            
            // Add new peers without duplicates
            for peer in config.peers {
                if !network_config.peers.contains(&peer) {
                    network_config.peers.push(peer);
                }
            }
            
            // Copy trusted peers from existing config
            network_config.trusted_peers = config.trusted_peers;
            
            // Adopt TLS setting from config
            network_config.use_tls = config.use_tls;
            
            println!("Network configured with {} peers", network_config.peers.len());
        } else {
            println!("{}", "Warning: Failed to configure network".yellow());
        }
    } else {
        println!("No existing configuration found, using defaults");
    }
    
    // Create communication channels with larger buffer for stability
    let (tx, mut rx) = mpsc::channel::<String>(1000);
    let running = Arc::new(Mutex::new(true));
    
    // Clone for thread use
    let running_clone = running.clone();
    
    // Clone the wallet address for the blockchain thread
    let default_wallet_clone = default_wallet.clone();
    
    // Add block generation monitoring - moved before message handler
    let last_block_time = Arc::new(Mutex::new(std::time::Instant::now()));
    let last_block_time_clone = last_block_time.clone();
    
    // Start blockchain in separate thread with error handling
    println!("Starting blockchain simulation...");
    let blockchain_handle = tokio::spawn(async move {
        // Start blockchain simulation with the wallet address
        run_blockchain(running_clone, default_wallet_clone, tx);
    });
    
    // Extract TLS setting before moving network_config
    let using_tls = network_config.use_tls;
    
    // Start RPC server with the correct parameters
    println!("Starting RPC server on port {}", rpc_port);
    let _rpc_handle = tokio::spawn(async move {
        // Configure and start the RPC server with network_config and the message receiver
        if let Err(e) = start_rpc_server(network_config).await {
            eprintln!("RPC server error: {}", e);
        }
    });
    
    // Update Kanari config with latest settings
    if let Some(mapping) = kanari_config.as_mapping_mut() {
        mapping.insert(Value::from("rpc_port"), Value::from(rpc_port));
        mapping.insert(Value::from("localhost_only"), Value::from(localhost_only));
        // Add wallet selection to configuration
        mapping.insert(Value::from("active_address"), Value::from(default_wallet.clone()));
    }
    
    // Save both configuration formats for consistency
    if let Err(e) = save_kanari_config(&kanari_config) {
        println!("{}", format!("Warning: Failed to save kanari config: {}", e).yellow());
    }
    
    // Create a config Value to save with save_config
    let mut config_mapping = serde_yaml::Mapping::new();
    config_mapping.insert(Value::from("rpc_port"), Value::from(rpc_port));
    config_mapping.insert(Value::from("localhost_only"), Value::from(localhost_only));
    config_mapping.insert(Value::from("active_address"), Value::from(default_wallet.clone()));
    
    if let Err(e) = save_config(&Value::Mapping(config_mapping)) {
        println!("{}", format!("Warning: Failed to save config: {}", e).yellow());
    }
    
    // Print server info
    println!("\n{}", "Kari Server is running!".bright_green().bold());
    println!("RPC endpoint: {}", format!("http://localhost:{}", rpc_port).bright_white());
    if localhost_only {
        println!("Mode: {}", "Localhost only".yellow());
    } else {
        println!("Mode: {}", "Network enabled".green());
    }
    println!("Connected peers: {}", peers_count.to_string().bright_white());
    println!("Using wallet: {}", default_wallet.bright_white());
    println!("TLS encryption: {}", if using_tls { "Enabled".green() } else { "Disabled".yellow() });
    println!("\nPress Ctrl+C to stop the server\n");
    
    // Process and handle messages from blockchain
    let message_handler = tokio::spawn(async move {
        // Clone for use within this closure
        let last_block_time = last_block_time_clone;
        
        while let Some(message) = rx.recv().await {
            // Check if message is JSON and handle appropriately
            if message.starts_with('{') && message.ends_with('}') {
                // It's likely JSON, try to parse and display formatted
                match serde_json::from_str::<serde_json::Value>(&message) {
                    Ok(json_value) => {
                        // Extract message type if available
                        if let Some(msg_type) = json_value.get("type").and_then(|v| v.as_str()) {
                            match msg_type {
                                "error" => {
                                    let error_msg = json_value.get("message")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Unknown error");
                                    println!("{}: {}", "Blockchain error".red().bold(), error_msg);
                                },
                                "block" => {
                                    let height = json_value.get("height")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    println!("{} {}", "New block:".green(), height);
                                    
                                    // Reset block generation monitoring timer when a block is received
                                    if let Ok(mut last_time) = last_block_time.lock() {
                                        *last_time = std::time::Instant::now();
                                    }
                                },
                                "transaction" => {
                                    println!("{}: {}", "Transaction".bright_blue(), 
                                        json_value.get("id").and_then(|v| v.as_str()).unwrap_or("unknown"));
                                },
                                _ => println!("Blockchain: {}", message),
                            }
                        } else {
                            // No type field, just print the message
                            println!("Blockchain: {}", message);
                        }
                    },
                    Err(_) => {
                        // Not valid JSON, print as-is
                        println!("Blockchain: {}", message);
                    }
                }
            } else {
                // Not JSON, print as-is
                println!("Blockchain: {}", message);
            }
        }
    });
    
    // Spawn a task to monitor block generation and report if no blocks are being generated
    let block_monitor = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            
            if let Ok(last_time) = last_block_time.lock() {
                let elapsed = last_time.elapsed();
                if elapsed > Duration::from_secs(60) {
                    println!("{}", "⚠️ Warning: No blocks have been generated in the last minute".yellow().bold());
                    println!("{}", "This might be due to:".yellow());
                    println!("  - No transactions to process");
                    println!("  - Network configuration issues (TLS settings)");
                    println!("  - Blockchain simulation not running properly");
                    println!("");
                    println!("{}", "Try restarting with '--localhost true' and '--no-tls' flags".bright_white());
                }
            }
        }
    });
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c event");
    
  match save_blockchain() {
      Ok(_) => println!("Blockchain state saved successfully"),
      Err(e) => println!("{}", format!("Warning: Failed to save blockchain state: {}", e).yellow()),
  }

    println!("{}", "Kari server stopped.".bright_green().bold());
    Ok(())
}
