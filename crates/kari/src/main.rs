use colored::Colorize;
use command::keytool_cli::handle_keytool_command;
use command::move_cli::handle_move_command;
use mona_crypto::{check_wallet_exists, list_wallet_files};
use std::io::{self, Write};
use std::process::exit;
use std::sync::{Arc, Mutex};

use command::public_cli::handle_public_command;

use network::NetworkConfig;
use panorama::simulation::run_blockchain;

use common::get_kari_dir;
use mona_blockchain::blockchain::{load_blockchain, save_blockchain};
use mona_blockchain::chain_id::CHAIN_ID;
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
        description: "Start a node on specific port (default: 30030)",
    },
    CommandInfo {
        name: "start --localhost",
        alias: None,
        description: "Start node in localhost-only mode (no external connections)",
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
        name: "certificate",
        alias: None,
        description: "Command has been deprecated",
    },
    CommandInfo {
        name: "version",
        alias: Some("--V"),
        description: "Display CLI version information",
    },
    CommandInfo {
        name: "help",
        alias: Some("--h"),
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
                    Some("--localhost") => {
                        localhost_only = true;
                        i += 1;
                    },
                    _ => {
                        eprintln!("Unknown argument: {}", args[i]);
                        display_help(true);
                        i += 1;
                    }
                }
            }
            
            start_node_with_peers(peers, port, localhost_only).await;
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
                .args(["/C", "start", "https://docs.kanari.site"])
                .spawn()
                .expect("Failed to open documentation");

            #[cfg(target_os = "linux")]
            Command::new("xdg-open")
                .arg("https://docs.kanari.site")
                .spawn()
                .expect("Failed to open documentation");

            #[cfg(target_os = "macos")]
            Command::new("open")
                .arg("https://docs.kanari.site")
                .spawn()
                .expect("Failed to open documentation");
        }
        _ => display_help(true),
    }
}

// Add a new function to start node with peer information
async fn start_node_with_peers(peers: Vec<String>, port: Option<u16>, localhost_only: bool) {
    // Check if any wallet exists first
    if !check_wallet_exists() {
        println!("{}", "No wallet found!".red());
        println!("Please create a wallet first using:");
        println!("{}", "kari keytool generate".green());
        exit(1);
    }

    // Load configuration with better error handling
    let mut config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Creating a new configuration...");
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
    };

    let _chain_id = config
        .get("chain_id")
        .and_then(|v| v.as_str())
        .unwrap_or(CHAIN_ID);

    // Check if the configuration already exists
    let network_config = if config.get("network_type").is_some()
        && config.get("rpc_port").is_some()
        && config.get("chain_id").is_some()
    {
        println!("Configuration already exists. Skipping configuration process.");
        
        // Use provided port if given, otherwise use configured port
        let rpc_port = match port {
            Some(p) => {
                println!("Using specified port: {}", p);
                p
            },
            None => match config.get("rpc_port").unwrap().as_u64() {
                Some(p) => p as u16,
                None => {
                    eprintln!("Invalid port in config, using default 30030");
                    30030
                }
            },
        };
        
        let chain_id = config
            .get("chain_id")
            .unwrap()
            .as_str()
            .unwrap_or(CHAIN_ID)
            .to_string();

        NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            port: rpc_port,
            peers: if localhost_only { vec![] } else { peers }, // No peers in localhost mode
            chain_id,
            max_connections: 100,
            api_enabled: true,
            localhost_only,
        }
    } else {
        // Call configure_network and get the NetworkConfig
        println!("No configuration found. Setting up new configuration...");
        match configure_network(CHAIN_ID) {
            Ok(mut config) => {
                // Update with any provided port or peers
                if let Some(p) = port {
                    println!("Using specified port: {}", p);
                    config.port = p;
                }
                
                if !localhost_only && !peers.is_empty() {
                    println!("Using specified peers: {:?}", peers);
                    config.peers = peers;
                } else {
                    config.peers = vec![];
                }
                
                config.localhost_only = localhost_only;
                config
            },
            Err(err) => {
                eprintln!("Error configuring network: {}", err);
                exit(1);
            }
        }
    };

    // Try to load blockchain with better error handling
    if let Err(e) = load_blockchain() {
        eprintln!("Warning: Failed to load blockchain: {}. A new blockchain will be created.", e);
    }
    
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
            serde_yaml::Value::String("rpc_port".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(network_config.port)),
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
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let running_clone = Arc::clone(&running);
    let address_clone = address.clone();

    // Spawn blockchain simulation task
    let blockchain_handle = tokio::spawn(async move {
        println!("Running blockchain simulation...");
        run_blockchain(running_clone, address_clone, tx);
    });

    // Get the local IP address
    let local_ip = match panorama::node::get_local_ip() {
        Some(ip) => ip,
        None => {
            eprintln!("Could not determine local IP address, using 127.0.0.1");
            "127.0.0.1".to_string()
        }
    };

    // Update RPC configuration to reflect localhost-only mode
    let rpc_config = NetworkConfig {
        node_address: if localhost_only { "127.0.0.1".to_string() } else { local_ip.clone() },
        port: network_config.port,
        peers: network_config.peers.clone(),
        chain_id: network_config.chain_id.clone(),
        max_connections: 100,
        api_enabled: true,
        localhost_only,
    };

    // Display IP address and port information for connecting nodes
    println!("{}", "Node network information:".bright_yellow());
    
    if localhost_only {
        println!("  LOCALHOST-ONLY MODE: Node is only accessible from this machine");
        println!("  RPC API:   127.0.0.1:{} (HTTP)", rpc_config.port);
        println!("  No external P2P connections will be allowed");
    } else {
        println!("  RPC API:   {}:{} (HTTP)", local_ip, rpc_config.port);
        println!("  P2P:       {}:51303", local_ip); 

        // Display peer connection information if applicable
        if !network_config.peers.is_empty() {
            println!("{}", "Node will connect to the following peers:".bright_yellow());
            for peer in &network_config.peers {
                println!("  - {}", peer.green());
            }
        } else {
            println!("{}", "Warning: No peers configured. Running in standalone mode.".yellow());
            println!("  To connect peers, use: kari start --peer <IP:PORT>");
        }
    }

    // Start RPC server with shutdown signal
    let rpc_handle = tokio::spawn(async move {
        println!("Starting RPC server on port {}...", rpc_config.port);
        
        // Try multiple times to start the server in case of port issues
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 3;
        
        while attempts < MAX_ATTEMPTS {
            match start_rpc_server(rpc_config.clone()).await {
                Ok(_) => {
                    println!("RPC server started successfully on port {}.", rpc_config.port);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    eprintln!("Failed to start RPC server (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                    
                    if attempts >= MAX_ATTEMPTS {
                        eprintln!("Failed to start RPC server after {} attempts. Exiting.", MAX_ATTEMPTS);
                        std::process::exit(1);
                    }
                    
                    // Try with a different port
                    let mut new_config = rpc_config.clone();
                    new_config.port += 1;
                    eprintln!("Trying with port {} instead...", new_config.port);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        
        // Keep this task alive until it's aborted
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            tokio::task::yield_now().await;
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

    // Display block status updates; break loop once shutdown is signaled
    loop {
        tokio::select! {
            Some(status) = rx.recv() => {
                println!("{}", status.bright_cyan());
            }
            _ = &mut shutdown_rx => {
                println!("Shutdown signal received. Stopping node...");
                *running.lock().unwrap() = false;
                break;
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
    // Abort the RPC server task explicitly before exiting
    rpc_handle.abort();
    std::process::exit(0);
}
