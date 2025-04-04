use colored::Colorize;
use command::keytool_cli::handle_keytool_command;
use command::move_cli::handle_move_command;
use std::io::{self, Write};
use std::process::exit;
use std::sync::{Arc, Mutex};

use command::public_cli::handle_public_command;
use key::{check_wallet_exists, list_wallet_files};
use network::{NetworkConfig, NetworkType};
use panorama::simulation::run_blockchain;

use common::get_kari_dir;
use panorama::blockchain::{load_blockchain, save_blockchain};
use panorama::chain_id::CHAIN_ID;
use panorama::config::{configure_network, load_config, save_config};
use rpc_api::start_rpc_server;
use std::process::Command;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

static VERSION: &str = env!("CARGO_PKG_VERSION");

struct CommandInfo {
    name: &'static str,
    alias: Option<&'static str>,
    description: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "start",
        alias: None,
        description: "Start a local Kari blockchain node",
    },
    CommandInfo {
        name: "start --peer <IP:PORT>",
        alias: None,
        description: "Start a node and connect to specified peer",
    },
    CommandInfo {
        name: "start --port <PORT>",
        alias: None,
        description: "Start a node on specific port (default: 30031)",
    },
    CommandInfo {
        name: "public",
        alias: None,
        description: "Manage Web3 public files and IPFS storage",
    },
    CommandInfo {
        name: "move",
        alias: None,
        description: "Execute and manage Move VM smart contracts",
    },
    CommandInfo {
        name: "keytool",
        alias: None,
        description: "Manage Kari accounts and cryptographic keys",
    },
    CommandInfo {
        name: "version",
        alias: Some("--V"),
        description: "Display CLI version information",
    },
    CommandInfo {
        name: "help",
        alias: Some("-h"),
        description: "Display this help message",
    },
    CommandInfo {
        name: "info",
        alias: Some("--i"),
        description: "Display information about the Kari node",
    },
];

fn display_help(show_error: bool) {
    if show_error {
        println!("\n{}", "ERROR: Invalid command".red().bold());
    }

    // Usage
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("kari <command> [options]\n");

    // Commands
    println!("{}", "COMMANDS:".bright_yellow().bold());

    let max_name_len = COMMANDS
        .iter()
        .map(|cmd| cmd.name.len() + cmd.alias.map_or(0, |a| a.len() + 2))
        .max()
        .unwrap_or(0);

    for cmd in COMMANDS {
        let name = match cmd.alias {
            Some(alias) => format!("{}, {}", cmd.name, alias),
            None => cmd.name.to_string(),
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
        Some("start") => {
            // Extract peer and port arguments
            let mut peers = Vec::new();
            let mut port = None;
            
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
                    },
                    Some("--port") => {
                        if let Some(port_str) = args.get(i + 1) {
                            match port_str.parse::<u16>() {
                                Ok(p) => {
                                    port = Some(p);
                                    i += 2;
                                },
                                Err(_) => {
                                    eprintln!("Error: Invalid port number");
                                    exit(1);
                                }
                            }
                        } else {
                            eprintln!("Error: --port requires a number argument");
                            exit(1);
                        }
                    },
                    _ => {
                        eprintln!("Unknown argument: {}", args[i]);
                        display_help(true);
                        i += 1;
                    }
                }
            }
            
            start_node_with_peers(peers, port).await;
        },
        Some("public") => {
            let _ = handle_public_command();
        }
        Some("move") => handle_move_command(),
        Some("keytool") => {
            let _ = handle_keytool_command();
        }
        Some("version") | Some("--V") => println!("CLI Version: {}", VERSION),
        Some("help") | Some("--h") => display_help(false),
        Some("info") | Some("--i") => {
            println!("{}", "Opening Kari documentation...".bright_yellow());
            #[cfg(target_os = "windows")]
            Command::new("cmd")
                .args(["/C", "start", "https://docs.kanari.network"])
                .spawn()
                .expect("Failed to open documentation");

            #[cfg(target_os = "linux")]
            Command::new("xdg-open")
                .arg("https://docs.kanari.network")
                .spawn()
                .expect("Failed to open documentation");

            #[cfg(target_os = "macos")]
            Command::new("open")
                .arg("https://docs.kanari.network")
                .spawn()
                .expect("Failed to open documentation");
        }
        _ => display_help(true),
    }
}

// Add a new function to start node with peer information
async fn start_node_with_peers(peers: Vec<String>, port: Option<u16>) {
    // Check if any wallet exists first
    if !check_wallet_exists() {
        println!("{}", "No wallet found!".red());
        println!("Please create a wallet first using:");
        println!("{}", "kari keytool generate".green());
        exit(1);
    }

    let mut config = load_config().expect("Failed to load configuration file");

    let _chain_id = config
        .get("chain_id")
        .and_then(|v| v.as_str())
        .unwrap_or(CHAIN_ID);

    // Check if the configuration already exists
    let network_config = if config.get("network_type").is_some()
        && config.get("rpc_port").is_some()
        && config.get("domain").is_some()
        && config.get("chain_id").is_some()
    {
        println!("Configuration already exists. Skipping configuration process.");
        let network_type = match config.get("network_type").unwrap().as_str().unwrap() {
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            "mainnet" => NetworkType::Mainnet,
            _ => unreachable!(),
        };
        
        // Use provided port if given, otherwise use configured port
        let rpc_port = match port {
            Some(p) => p,
            None => config.get("rpc_port").unwrap().as_u64().unwrap() as u16,
        };
        
        let domain = config.get("domain").unwrap().as_str().unwrap().to_string();
        let chain_id = config
            .get("chain_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain: domain,
            port: rpc_port,
            peers: peers,  // Use provided peers
            chain_id,
            max_connections: 100,
            api_enabled: true,
            network_type,
        }
    } else {
        // Call configure_network and get the NetworkConfig
        match configure_network(CHAIN_ID) {
            Ok(mut config) => {
                // Update with any provided port or peers
                if let Some(p) = port {
                    config.port = p;
                }
                
                if !peers.is_empty() {
                    config.peers = peers;
                }
                
                config
            },
            Err(err) => {
                eprintln!("Error configuring network: {}", err);
                exit(1);
            }
        }
    };

    let _ = load_blockchain();
    let running = Arc::new(Mutex::new(true));

    // Load address with validation
    let address = match config.get("address").and_then(|v| v.as_str()) {
        Some(address) => {
            // Verify wallet file exists for this address
            if !std::path::Path::new(
                &get_kari_dir()
                    .join("wallets")
                    .join(format!("{}.enc", address)),
            )
            .exists()
            {
                // Try to find any existing wallet
                match list_wallet_files() {
                    Ok(wallets) if !wallets.is_empty() => {
                        // Access first element of tuple (filename)
                        let first_wallet = wallets[0].0.trim_end_matches(".enc").to_string();
                        println!("Using existing wallet as address: {}", first_wallet.green());

                        // Convert config to Map to modify it
                        if let serde_yaml::Value::Mapping(ref mut map) = config {
                            map.insert(
                                serde_yaml::Value::String("address".to_string()),
                                serde_yaml::Value::String(first_wallet.clone()),
                            );
                            save_config(&config).expect("Failed to save configuration");
                        }

                        first_wallet
                    }
                    _ => {
                        println!("{}", "No valid wallets found!".red());
                        println!("Please create a wallet first using:");
                        println!("{}", "kari keytool generate".green());
                        exit(1);
                    }
                }
            } else {
                address.to_string()
            }
        }
        None => {
            // Try to find any existing wallet
            match list_wallet_files() {
                Ok(wallets) if !wallets.is_empty() => {
                    let first_wallet = wallets[0].0.trim_end_matches(".enc").to_string();
                    println!(
                        "Setting address to existing wallet: {}",
                        first_wallet.green()
                    );

                    // Update config with new address using serde_yaml::Value
                    if let serde_yaml::Value::Mapping(ref mut map) = config {
                        map.insert(
                            serde_yaml::Value::String("address".to_string()),
                            serde_yaml::Value::String(first_wallet.clone()),
                        );
                        save_config(&config).expect("Failed to save configuration");
                    }

                    first_wallet
                }
                _ => {
                    println!("{}", "No wallets found!".red());
                    println!("Please create a wallet first using:");
                    println!("{}", "kari keytool create".green());
                    exit(1);
                }
            }
        }
    };

    let final_config = serde_yaml::Value::Mapping({
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("chain_id".to_string()),
            serde_yaml::Value::String(network_config.chain_id.clone()),
        );
        map.insert(
            serde_yaml::Value::String("network_type".to_string()),
            serde_yaml::Value::String(network_config.network_type.to_string()),
        );
        map.insert(
            serde_yaml::Value::String("rpc_port".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(network_config.port)),
        );
        map.insert(
            serde_yaml::Value::String("domain".to_string()),
            serde_yaml::Value::String(network_config.domain.clone()),
        );
        map.insert(
            serde_yaml::Value::String("address".to_string()),
            serde_yaml::Value::String(address.clone()),
        );
        
        // Add peers to configuration if specified
        if !network_config.peers.is_empty() {
            let peers_array = network_config.peers.iter()
                .map(|peer| serde_yaml::Value::String(peer.clone()))
                .collect::<Vec<_>>();
            
            map.insert(
                serde_yaml::Value::String("peers".to_string()),
                serde_yaml::Value::Sequence(peers_array),
            );
        }
        
        map
    });
    save_config(&final_config).expect("Failed to save configuration");

    if address.is_empty() {
        println!("Please generate an address first using the 'kari keytool' command.");
        exit(1);
    }

    println!("Using existing address: {}", address.green());
    *running.lock().unwrap() = true;
    println!("{}", "Starting blockchain...".green());

    // Create a channel for block status updates
    let (tx, mut rx) = mpsc::channel::<String>(100);
    
    // Create a oneshot channel for shutdown signaling
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

    let running_clone = Arc::clone(&running);
    let address_clone = address.clone();

    // Spawn blockchain simulation task
    let blockchain_handle = tokio::spawn(async move {
        println!("Running blockchain simulation...");
        run_blockchain(running_clone, address_clone, tx);
    });

    // Update RPC configuration
    let rpc_config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        domain: network_config.domain.clone(),
        port: network_config.port,
        peers: network_config.peers.clone(),
        chain_id: network_config.chain_id.clone(),
        max_connections: 100,
        api_enabled: true,
        network_type: network_config.network_type,
    };

    // Display peer connection information if applicable
    if !network_config.peers.is_empty() {
        println!("{}", "Node will connect to the following peers:".bright_yellow());
        for peer in &network_config.peers {
            println!("  - {}", peer.green());
        }
    }

    let _rpc_handle = tokio::spawn(async move {
        println!("Starting RPC server on port {}...", rpc_config.port);
        
        // Create a shutdown signal for RPC server - fix the type annotation
        let (_rpc_shutdown_tx, rpc_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        
        // Pass shutdown channel to RPC server
        tokio::select! {
            _ = start_rpc_server(rpc_config) => {
                println!("RPC server stopped.");
            }
            _ = rpc_shutdown_rx => {
                println!("RPC server received shutdown signal.");
            }
        }
    });

    // This task will handle the Enter keypress
    println!("{}", "Block status will be shown below. Press Enter to stop the node.".yellow());
    io::stdout().flush().unwrap();

    // Spawn a task to listen for Enter key
    tokio::spawn(async move {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                println!("Received shutdown request...");
                // Signal shutdown
                let _ = shutdown_tx.send(());
            }
            Err(e) => {
                println!("Error reading input: {}", e);
                let _ = shutdown_tx.send(());
            }
        }
    });

    // Use a separate boolean for tracking shutdown status
    let mut shutdown_requested = false;

    // Display block status updates while waiting for shutdown signal
    while !shutdown_requested {
        tokio::select! {
            Some(status) = rx.recv() => {
                println!("{}", status.bright_cyan());
            }
            _ = &mut shutdown_rx => {
                println!("Shutdown signal received. Stopping node...");
                // Set running to false to stop blockchain
                *running.lock().unwrap() = false;
                shutdown_requested = true;
            }
        }
    }

    // Give some time for the blockchain to shutdown gracefully
    println!("{}", "Stopping blockchain...".red());
    sleep(Duration::from_secs(1)).await;

    // Ensure blockchain state is saved
    let _ = save_blockchain();
    
    // Wait for blockchain task to complete (with timeout)
    match tokio::time::timeout(Duration::from_secs(5), blockchain_handle).await {
        Ok(_) => println!("Blockchain stopped gracefully"),
        Err(_) => println!("Blockchain shutdown timed out, forcing exit"),
    }

    // Force exit (needed because RPC server might be hanging)
    println!("Node stopped. Exiting...");
    std::process::exit(0);
}

// Replace the original start_node function with a call to start_node_with_peers
async fn start_node() {
    start_node_with_peers(Vec::new(), None).await
}
