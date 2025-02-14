
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::process::exit;
use colored::Colorize;
use command::keytool_cli::handle_keytool_command;
use command::move_cli::handle_move_command;

use command::public_cli::handle_public_command;
use k2::simulation::run_blockchain;
use key::{check_wallet_exists, list_wallet_files};
use network::{NetworkConfig, NetworkType};



use rpc_api::start_rpc_server;
use serde_json::json;
use k2::blockchain::{get_kari_dir, load_blockchain, save_blockchain, BALANCES};
use k2::chain_id::CHAIN_ID;
use k2::config::{configure_network, load_config, save_config};
use std::process::Command;
use semver::Version;

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
        description: "Start a local Kari blockchain node" 
    },
    CommandInfo { 
        name: "public", 
        alias: None, 
        description: "Manage Web3 public files and IPFS storage" 
    },
    CommandInfo { 
        name: "move", 
        alias: None, 
        description: "Execute and manage Move VM smart contracts" 
    },
    CommandInfo { 
        name: "keytool", 
        alias: None, 
        description: "Manage Kari accounts and cryptographic keys" 
    },
    CommandInfo { 
        name: "update", 
        alias: Some("--up"), 
        description: "Update Kari tools to latest version from GitHub" 
    },
    CommandInfo { 
        name: "version", 
        alias: Some("--V"), 
        description: "Display CLI version information" 
    },
    CommandInfo {
        name: "help",
        alias: Some("-h"),
        description: "Display this help message"
    },
    CommandInfo { 
        name: "info", 
        alias: Some("--i"), 
        description: "Display information about the Kari node" 
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
        Some("public") => {
            let _ = handle_public_command();
        },
        Some("move") => handle_move_command(),
        Some("keytool") => {
            let _ = handle_keytool_command();
        },
        Some("update") | Some("--up") => {
            if let Err(err) = handle_update().await {
                eprintln!("Update failed: {}", err);
                exit(1);
            }
        },
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
        },
        _ => display_help(true),
    }
}

// Start the Kari node
async fn start_node() {

        // Check if any wallet exists first
        if !check_wallet_exists() {
            println!("{}", "No wallet found!".red());
            println!("Please create a wallet first using:");
            println!("{}", "kari keytool generate".green());
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
        let domain = config.get("domain").unwrap().as_str().unwrap().to_string();
        let chain_id = config.get("chain_id").unwrap().as_str().unwrap().to_string();

        NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain,
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


    let _ = load_blockchain();
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
                        println!("{}", "kari keytool generate".green());
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

    let final_config = json!({
        "chain_id": network_config.chain_id,
        "network_type": network_config.network_type.to_string(),
        "rpc_port": network_config.port,
        "domain": network_config.domain,
        "miner_address": miner_address,
    });
    save_config(&final_config).expect("Failed to save configuration");

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
        let _ = save_blockchain();
        break;
    }
}

// Add new function for update handling
async fn handle_update() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Checking for updates...".bright_yellow());
    
    // Check current version
    let current_version = Version::parse(VERSION)?;
    
    // Fetch latest version from GitHub
    let latest_version = fetch_latest_version().await?;
    
    if latest_version > current_version {
        println!("New version available: {} -> {}", 
            current_version.to_string().red(),
            latest_version.to_string().green()
        );
        
        println!("{}", "Updating Kari tools...".bright_yellow());
        
        // Run git pull to update
        let status = Command::new("git")
            .args(&["pull", "https://github.com/kanari-network/kanari-sdk.git", "kanari-sdk"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?;

        if status.success() {
            // Rebuild after update
            let build_status = Command::new("cargo")
                .args(&["build", "--release"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .status()?;

            if build_status.success() {
                println!("{}", "Update completed successfully!".green());
                println!("Please restart Kari to use the new version");
            } else {
                println!("{}", "Failed to rebuild after update".red());
            }
        } else {
            println!("{}", "Update failed".red());
        }
    } else {
        println!("{}", "Already at latest version".green());
    }
    
    Ok(())
}

// Add helper function to fetch latest version
async fn fetch_latest_version() -> Result<Version, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/kanari-network/kanari-sdk/releases/latest")
        .header("User-Agent", "Kari-CLI")
        .send()
        .await?;
    
    let release = response.json::<serde_json::Value>().await?;
    let version = release["tag_name"]
        .as_str()
        .ok_or("No version tag found")?
        .trim_start_matches('v');
    
    Ok(Version::parse(version)?)
}