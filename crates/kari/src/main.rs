
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

use k2::blockchain::{get_kari_dir, load_blockchain, save_blockchain, BALANCES};
use k2::chain_id::CHAIN_ID;
use k2::config::{configure_network, load_config, save_config};
use std::process::Command;


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
        // let domain = config.get("domain").unwrap().as_str().unwrap().to_string();
        let chain_id = config.get("chain_id").unwrap().as_str().unwrap().to_string();

        NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain: "mainnet.kanari.network".to_string(),
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

    // Load address with validation
    let address = match config.get("address").and_then(|v| v.as_str()) {
        Some(address) => {
            // Verify wallet file exists for this address
            if !std::path::Path::new(&get_kari_dir().join("wallets").join(format!("{}.toml", address))).exists() {
                // Try to find any existing wallet
                match list_wallet_files() {
                    Ok(wallets) if !wallets.is_empty() => {
                        // Access first element of tuple (filename)
                        let first_wallet = wallets[0].0.trim_end_matches(".toml").to_string();
                        println!("Using existing wallet as address: {}", first_wallet.green());
                        
                        // Convert config to Map to modify it
                        if let serde_yaml::Value::Mapping(ref mut map) = config {
                            map.insert(
                                serde_yaml::Value::String("address".to_string()),
                                serde_yaml::Value::String(first_wallet.clone())
                            );
                            save_config(&config).expect("Failed to save configuration");
                        }
                        
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
                    println!("Setting address to existing wallet: {}", first_wallet.green());
                    
                    // Update config with new address using serde_yaml::Value
                    if let serde_yaml::Value::Mapping(ref mut map) = config {
                        map.insert(
                            serde_yaml::Value::String("address".to_string()),
                            serde_yaml::Value::String(first_wallet.clone())
                        );
                        save_config(&config).expect("Failed to save configuration");
                    }
                    
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

    let final_config = serde_yaml::Value::Mapping({
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("chain_id".to_string()),
            serde_yaml::Value::String(network_config.chain_id)
        );
        map.insert(
            serde_yaml::Value::String("network_type".to_string()),
            serde_yaml::Value::String(network_config.network_type.to_string().clone())
        );
        map.insert(
            serde_yaml::Value::String("rpc_port".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(network_config.port))
        );
        map.insert(
            serde_yaml::Value::String("domain".to_string()),
            serde_yaml::Value::String(network_config.domain)
        );
        map.insert(
            serde_yaml::Value::String("address".to_string()),
            serde_yaml::Value::String(address.clone())
        );
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
    
    let running_clone = Arc::clone(&running);
    let address_clone = address.clone();
    

    
    // Spawn blockchain simulation task
    tokio::spawn(async move {
        println!("Running blockchain simulation...");
        run_blockchain(running_clone, address_clone);
    });
    
    // Wait for shutdown signal
    println!("{} to stop the node", "Press Enter".yellow());
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    // Graceful shutdown
    println!("{}", "Stopping blockchain...".red());
    *running.lock().unwrap() = false;
    let _ = save_blockchain();
}

