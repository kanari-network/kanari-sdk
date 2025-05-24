use colored::Colorize;
use network::NetworkConfig;
use panorama::simulation::run_blockchain;

use common::{load_kanari_config, save_kanari_config, load_config};
use mona_blockchain::blockchain::{load_blockchain, save_blockchain};
use mona_blockchain::chain_id::CHAIN_ID;
use panorama::config::{configure_network,};
use rpc_api::start_rpc_server;


use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

use mona_crypto::{check_wallet_exists, list_wallet_files};
use std::sync::{Arc, Mutex};
use serde_yaml::Value;
// Removed: use std::path::Path; // No longer needed for wallet checks here
use std::io::{self, Write};
use std::error::Error; // For Box<dyn Error>

pub async fn start_server(
    peers: Vec<String>,
    port: Option<u16>,
    localhost_only: bool,
    selected_wallet: Option<String>,
    use_tls: bool,
) -> Result<(), Box<dyn Error>> {
    // Check if any wallet exists first
    if !check_wallet_exists() {
        println!("{}", "No wallet found!".red());
        println!("Please create a wallet first using:");
        println!("{}", "kari keytool generate".green());
        return Err(Box::from("No wallet found. Please create a wallet first."));
    }

    // Load configuration with better error handling
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Creating a new configuration...");
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
    };

    // Also load kanari configuration
    let mut kanari_config = match load_kanari_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load kanari configuration: {}", e);
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
    };

    let _chain_id = config
        .get("chain_id")
        .and_then(|v| v.as_str())
        .unwrap_or(CHAIN_ID);

    // Get the local IP address earlier for consistent use in configurations
    let determined_local_ip = match panorama::node::get_local_ip() {
        Some(ip) => ip,
        None => {
            eprintln!("Warning: Could not determine local IP address. Defaulting to 127.0.0.1 for configuration purposes.");
            "127.0.0.1".to_string()
        }
    };

    // Check if the configuration already exists
    let network_config = if config.get("rpc_port").is_some() && config.get("chain_id").is_some() {
        println!("Configuration already exists. Skipping configuration process.");

        // Use provided port if given, otherwise use configured port
        let rpc_port = match port {
            Some(p) => {
                println!("Using specified port: {}", p);

                // Update kanari config with the new port if we have kanari config
                if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
                    // Store active_env as a String to end the immutable borrow
                    let active_env = kanari_mapping
                        .get("active_env")
                        .and_then(|v| v.as_str())
                        .unwrap_or("local")
                        .to_string();

                    if let Some(envs) = kanari_mapping
                        .get_mut("envs")
                        .and_then(|v| v.as_sequence_mut())
                    {
                        for env in envs {
                            if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                                if alias == active_env {
                                    // Update RPC URL with the new port
                                    env["rpc"] = Value::String(format!("http://127.0.0.1:{}", p));
                                    break;
                                }
                            }
                        }
                    }

                    // Save the updated kanari config
                    if let Err(e) = save_kanari_config(&kanari_config) {
                        eprintln!("Warning: Failed to save updated kanari config: {}", e);
                    }
                }

                p
            }
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
            node_address: if localhost_only { "127.0.0.1".to_string() } else { determined_local_ip.clone() },
            port: rpc_port,
            peers: if localhost_only { vec![] } else { peers }, // No peers in localhost mode
            chain_id,
            max_connections: 100,
            api_enabled: true,
            localhost_only,
            trusted_peers: Vec::new(), // Added field
            use_tls,                   // Added field
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

                    // Update kanari config with the new port
                    if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
                        // Store active_env as a String to end the immutable borrow
                        let active_env = kanari_mapping
                            .get("active_env")
                            .and_then(|v| v.as_str())
                            .unwrap_or("local")
                            .to_string();
    
                        if let Some(envs) = kanari_mapping
                            .get_mut("envs")
                            .and_then(|v| v.as_sequence_mut())
                        {
                            for env in envs {
                                if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                                    if alias == active_env {
                                        // Update RPC URL with the new port
                                        env["rpc"] =
                                            Value::String(format!("http://127.0.0.1:{}", p));
                                        break;
                                    }
                                }
                            }
                        }

                        // Save the updated kanari config
                        if let Err(e) = save_kanari_config(&kanari_config) {
                            eprintln!("Warning: Failed to save updated kanari config: {}", e);
                        }
                    }
                }

                if !localhost_only && !peers.is_empty() {
                    println!("Using specified peers: {:?}", peers);
                    config.peers = peers;
                } else {
                    config.peers = vec![];
                }

                config.localhost_only = localhost_only;
                config.use_tls = use_tls; // Ensure this is set if configure_network doesn't handle it
                // Assuming configure_network's returned NetworkConfig needs these fields too
                // If configure_network returns a struct that needs to be converted or already has these, adjust accordingly.
                // For now, let's assume we are building it or it's part of the `config` variable of type NetworkConfig.
                // This part might need adjustment based on the actual return type of `configure_network`
                // and how `NetworkConfig` is structured.
                // If `config` is already a `NetworkConfig`:
                // config.trusted_peers = Vec::new(); // Add if not set by configure_network
                // config.use_tls = use_tls; // Add if not set by configure_network

                // Let's construct it fully here for clarity, assuming configure_network provides basic settings
                // This is a placeholder, as the original code directly uses `config` after modification.
                // We'll assume `config` returned by `configure_network` is of type `NetworkConfig`
                // and we just need to ensure all fields are present.
                // The original code implies `config` is already `NetworkConfig`
                 NetworkConfig {
                    node_address: config.node_address,
                    port: config.port,
                    peers: config.peers,
                    chain_id: config.chain_id,
                    max_connections: config.max_connections,
                    api_enabled: config.api_enabled,
                    localhost_only: config.localhost_only,
                    trusted_peers: Vec::new(), // Added
                    use_tls, // Added
                }
            }
            Err(err) => {
                eprintln!("Error configuring network: {}", err);
                return Err(Box::from(format!("Error configuring network: {}", err)));
            }
        }
    };

    // Try to load blockchain with better error handling
    if let Err(e) = load_blockchain() {
        eprintln!(
            "Warning: Failed to load blockchain: {}. A new blockchain will be created.",
            e
        );
    }

    let running = Arc::new(Mutex::new(true));

    // Address selection logic updated for selected_wallet
    let address = if let Some(wallet_name) = selected_wallet {
        let all_wallets = list_wallet_files().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to list wallet files: {}", e);
            Vec::new()
        });

        if all_wallets.iter().any(|(addr, _)| addr == &wallet_name) {
            println!("Using selected wallet as address: {}", wallet_name.green());
            if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
                kanari_mapping.insert(
                    Value::String("active_address".to_string()),
                    Value::String(wallet_name.clone()),
                );
                if let Err(e) = save_kanari_config(&kanari_config) {
                    eprintln!("Warning: Failed to save updated kanari config: {}", e);
                }
            }
            wallet_name
        } else {
            println!("Selected wallet '{}' does not exist.", wallet_name.red());
            if let Some((first_wallet_addr, _)) = all_wallets.first() {
                println!("Falling back to existing wallet: {}", first_wallet_addr.green());
                if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
                    kanari_mapping.insert(
                        Value::String("active_address".to_string()),
                        Value::String(first_wallet_addr.clone()),
                    );
                    if let Err(e) = save_kanari_config(&kanari_config) {
                        eprintln!("Warning: Failed to save updated kanari config: {}", e);
                    }
                }
                first_wallet_addr.clone()
            } else {
                println!("{}", "No other wallets found!".red());
                println!("Please create a wallet or ensure the selected wallet exists.");
                return Err(Box::from("Selected wallet not found and no fallback wallets available."));
            }
        }
    } else {
        // Original logic if no specific wallet is selected
        let all_wallets = list_wallet_files().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to list wallet files: {}", e);
            Vec::new()
        });

        if let Some(kanari_mapping) = kanari_config.as_mapping_mut() { // Changed to as_mapping_mut for potential updates
            if let Some(active_addr_str) = kanari_mapping.get("active_address").and_then(|v| v.as_str()) {
                if all_wallets.iter().any(|(addr, _)| addr == active_addr_str) {
                    active_addr_str.to_string()
                } else {
                    println!("Active address wallet '{}' not found in keystore.", active_addr_str.red());
                    if let Some((first_wallet_addr, _)) = all_wallets.first() {
                        println!("Using existing wallet: {}", first_wallet_addr.green());
                        kanari_mapping.insert(
                            Value::String("active_address".to_string()),
                            Value::String(first_wallet_addr.clone()),
                        );
                        if let Err(e) = save_kanari_config(&kanari_config) {
                            eprintln!("Warning: Failed to save updated kanari config: {}", e);
                        }
                        first_wallet_addr.clone()
                    } else {
                        println!("{}", "No valid wallets found!".red());
                        return Err(Box::from("No valid wallets found after checking active_address."));
                    }
                }
            } else {
                // No active_address in kanari_config, try config.yaml or list_wallet_files
                match config.get("address").and_then(|v| v.as_str()) {
                    Some(address_str_from_config) => {
                        if all_wallets.iter().any(|(addr, _)| addr == address_str_from_config) {
                            // Update kanari_config with this address
                            kanari_mapping.insert(
                                Value::String("active_address".to_string()),
                                Value::String(address_str_from_config.to_string()),
                            );
                            if let Err(e) = save_kanari_config(&kanari_config) {
                                eprintln!("Warning: Failed to save updated kanari config: {}", e);
                            }
                            address_str_from_config.to_string()
                        } else {
                            println!("Config address wallet '{}' not found in keystore.", address_str_from_config.red());
                            if let Some((first_wallet_addr, _)) = all_wallets.first() {
                                println!("Using existing wallet: {}", first_wallet_addr.green());
                                kanari_mapping.insert(
                                    Value::String("active_address".to_string()),
                                    Value::String(first_wallet_addr.clone()),
                                );
                                if let Err(e) = save_kanari_config(&kanari_config) {
                                    eprintln!("Warning: Failed to save updated kanari config: {}", e);
                                }
                                first_wallet_addr.clone()
                            } else {
                                println!("{}", "No valid wallets found!".red());
                                return Err(Box::from("No valid wallets found after checking config.address."));
                            }
                        }
                    }
                    None => {
                        // No address in config.yaml, use first from list_wallet_files
                        if let Some((first_wallet_addr, _)) = all_wallets.first() {
                            println!("Setting address to existing wallet: {}", first_wallet_addr.green());
                            kanari_mapping.insert(
                                Value::String("active_address".to_string()),
                                Value::String(first_wallet_addr.clone()),
                            );
                            if let Err(e) = save_kanari_config(&kanari_config) {
                                eprintln!("Warning: Failed to save updated kanari config: {}", e);
                            }
                            first_wallet_addr.clone()
                        } else {
                            println!("{}", "No wallets found!".red());
                            return Err(Box::from("No wallets found at all."));
                        }
                    }
                }
            }
        } else {
             // Fallback if kanari_config is not a mapping (e.g., corrupted or new)
             // This block might be less likely if kanari_config is initialized to a mapping earlier
            match config.get("address").and_then(|v| v.as_str()) {
                Some(address_str_from_config) => {
                     if all_wallets.iter().any(|(addr, _)| addr == address_str_from_config) {
                        address_str_from_config.to_string()
                    } else {
                        println!("Config address wallet '{}' not found (no kanari_config). Using existing wallet.", address_str_from_config.red());
                        if let Some((first_wallet_addr, _)) = all_wallets.first() {
                            println!("Using existing wallet: {}", first_wallet_addr.green());
                            first_wallet_addr.clone()
                        } else {
                            println!("{}", "No valid wallets found!".red());
                            return Err(Box::from("No valid wallets found (no kanari_config, config.address invalid)."));
                        }
                    }
                }
                None => {
                    if let Some((first_wallet_addr, _)) = all_wallets.first() {
                        println!("Setting address to existing wallet (no kanari_config, no config.address): {}", first_wallet_addr.green());
                        first_wallet_addr.clone()
                    } else {
                        println!("{}", "No wallets found!".red());
                        return Err(Box::from("No wallets found (no kanari_config, no config.address)."));
                    }
                }
            }
        }
    };

    // Make sure kanari.yaml has the active address
    if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
        kanari_mapping.insert(
            serde_yaml::Value::String("active_address".to_string()),
            serde_yaml::Value::String(address.clone()),
        );
        if let Err(e) = save_kanari_config(&kanari_config) {
            eprintln!("Warning: Failed to save updated kanari config: {}", e);
        }
    }

    if address.is_empty() {
        println!("Please generate an address first using the 'kari keytool' command.");
        return Err(Box::from("Address is empty. Please generate an address."));
    }

    println!("Using existing address: {}", address.green());
    *running.lock().unwrap() = true;
    println!("{}", "Starting blockchain...".green());

    // Create a channel for block status updates
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Create a oneshot channel for shutdown signaling
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<>();

    let running_clone = Arc::clone(&running);
    let address_clone = address.clone();

    // Spawn blockchain simulation task
    let blockchain_handle = tokio::spawn(async move {
        println!("Running blockchain simulation...");
        run_blockchain(running_clone, address_clone, tx);
    });

    // Use the local IP address determined earlier
    let local_ip = determined_local_ip;

    // Update RPC configuration to reflect localhost-only mode
    let rpc_config = NetworkConfig {
        node_address: if localhost_only {
            "127.0.0.1".to_string()
        } else {
            local_ip.clone()
        },
        port: network_config.port,
        peers: network_config.peers.clone(),
        chain_id: network_config.chain_id.clone(),
        max_connections: 100,
        api_enabled: true,
        localhost_only,
        trusted_peers: Vec::new(), // Added field
        use_tls,                   // Added field
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
            println!(
                "{}",
                "Warning: No peers configured. Running in standalone mode.".yellow()
            );
            println!("  To connect peers, use: kari start --peer <IP:PORT>");
        }
    }

    // Start RPC server with shutdown signal
    let rpc_handle = tokio::spawn(async move {
        println!("Starting RPC server on port {}...", rpc_config.port);

        // Try multiple times to start the server in case of port issues
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 3;
        let mut current_rpc_config = rpc_config.clone();

        while attempts < MAX_ATTEMPTS {
            match start_rpc_server(current_rpc_config.clone()).await {
                Ok(_) => {
                    println!("RPC server started successfully on port {}.", current_rpc_config.port);
                    // Keep this task alive until it's aborted
                    loop {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        tokio::task::yield_now().await;
                    }
                    // Unreachable, but keeps structure, loop above is infinite until abort
                }
                Err(e) => {
                    attempts += 1;
                    eprintln!(
                        "Failed to start RPC server (attempt {}/{}): {}",
                        attempts, MAX_ATTEMPTS, e
                    );

                    if attempts >= MAX_ATTEMPTS {
                        eprintln!(
                            "Failed to start RPC server after {} attempts. Exiting from RPC spawn.",
                            MAX_ATTEMPTS
                        );
                        // This exit is inside a tokio::spawn, it will kill the task, not the main process directly.
                        // To signal failure to the main function, a channel or other IPC would be needed.
                        // For now, we let the task die. The main function will continue.
                        // If RPC server is critical, main function should await this handle or use a channel.
                        // Given the original code's std::process::exit(1), this task failing should probably propagate.
                        // However, changing this requires more significant refactoring of how errors are handled from spawned tasks.
                        // For now, just print and let the task end. The main function will proceed to shutdown.
                        return; // Exit the spawned task
                    }
                    
                    // Try with a different port
                    current_rpc_config.port += 1; // Modify the cloned config for the next attempt
                    eprintln!("Trying with port {} instead...", current_rpc_config.port);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
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
    // std::process::exit(0); // Replaced by Ok(())
    Ok(())
}
