use std::sync::{Arc, Mutex};
use std::process::exit;
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

// Import the key crate for wallet functionality
pub use key::{
    check_wallet_exists, list_wallet_files, generate_karix_address, save_wallet, 
    load_wallet, set_selected_wallet, import_from_seed_phrase, import_from_private_key, 
    WalletError, Wallet
};
use mona_types::address::Address;

pub use k2::blockchain::{get_kari_dir, load_blockchain, save_blockchain};
pub use k2::chain_id::CHAIN_ID;
pub use k2::config::{configure_network, load_config, save_config};
pub use k2::simulation::run_blockchain;
pub use network::{NetworkConfig, NetworkType};


/// Error type for node operations
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Wallet error: {0}")]
    WalletError(#[from] WalletError),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;

/// Node configuration
pub struct NodeConfig {
    pub address: String,
    pub network: NetworkConfig,
    pub enable_rpc: bool,
    pub enable_api: bool,
}

/// Starts a blockchain node with the provided configuration
pub async fn start_node(config: Option<NodeConfig>) -> Result<()> {
    // Check if any wallet exists
    if !check_wallet_exists() {
        println!("{}", "No wallet found!".red());
        println!("Please create a wallet first using:");
        println!("{}", "kari keytool generate".green());
        return Err(NodeError::ConfigError("No wallet found".to_string()));
    }

    // Load or create config
    let node_config = if let Some(cfg) = config {
        cfg
    } else {
        setup_default_configuration()?
    };

    // Load blockchain state
    let _ = load_blockchain();
    let running = Arc::new(Mutex::new(true));
    
    println!("Using address: {}", node_config.address.green());
    *running.lock().unwrap() = true;
    println!("{}", "Starting blockchain...".green());
    
    // Start blockchain in background
    let running_clone = Arc::clone(&running);
    let address_clone = node_config.address.clone();
    
    // Run blockchain simulation
    std::thread::spawn(move || {
        println!("Running blockchain simulation...");
        run_blockchain(running_clone, address_clone);
    });
    
    // Return the running flag for the caller to manage lifecycle
    Ok(())
}

/// Stops the blockchain node and saves state
pub fn stop_node() -> Result<()> {
    println!("{}", "Stopping blockchain...".red());
    
    // Save blockchain state before exit - correctly handle the error type
    save_blockchain().map_err(|e| NodeError::StorageError(e.to_string()))?;
    
    Ok(())
}

/// Setup default configuration using interactive prompts or environment
fn setup_default_configuration() -> Result<NodeConfig> {
    let mut config = load_config()
        .map_err(|e| NodeError::ConfigError(format!("Failed to load config: {}", e)))?;

    let chain_id = config.get("chain_id").and_then(|v| v.as_str()).unwrap_or(CHAIN_ID);

    // Check if configuration exists
    let network_config = if config.get("network_type").is_some() 
                           && config.get("rpc_port").is_some() 
                           && config.get("domain").is_some() 
                           && config.get("chain_id").is_some() {
        println!("Configuration already exists. Using existing configuration.");
        
        let network_type = match config.get("network_type").unwrap().as_str().unwrap() {
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            "mainnet" => NetworkType::Mainnet,
            _ => return Err(NodeError::ConfigError("Invalid network type".to_string())),
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
        // Configure network interactively
        match configure_network(chain_id) {
            Ok(config) => config,
            Err(err) => {
                return Err(NodeError::ConfigError(format!("Error configuring network: {}", err)));
            }
        }
    };

    // Get wallet address
    let address = get_or_create_wallet_address(&mut config)?;
    
    // Save final config
    let final_config = serde_yaml::Value::Mapping({
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("chain_id".to_string()),
            serde_yaml::Value::String(network_config.chain_id.clone())
        );
        map.insert(
            serde_yaml::Value::String("network_type".to_string()),
            serde_yaml::Value::String(network_config.network_type.to_string())
        );
        map.insert(
            serde_yaml::Value::String("rpc_port".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(network_config.port))
        );
        map.insert(
            serde_yaml::Value::String("domain".to_string()),
            serde_yaml::Value::String(network_config.domain.clone())
        );
        map.insert(
            serde_yaml::Value::String("address".to_string()),
            serde_yaml::Value::String(address.clone())
        );
        map
    });
    
    save_config(&final_config)
        .map_err(|e| NodeError::ConfigError(format!("Failed to save config: {}", e)))?;

    Ok(NodeConfig {
        address,
        network: network_config,
        enable_rpc: true,
        enable_api: true,
    })
}

/// Get an existing wallet address or create a new one
fn get_or_create_wallet_address(config: &mut serde_yaml::Value) -> Result<String> {
    match config.get("address").and_then(|v| v.as_str()) {
        Some(address) => {
            // Verify wallet file exists for this address
            if !Path::new(&get_kari_dir().join("wallets").join(format!("{}.enc", address))).exists() {
                // Try to find any existing wallet
                match list_wallet_files() {
                    Ok(wallets) if !wallets.is_empty() => {
                        // Use first available wallet
                        let first_wallet = wallets[0].0.trim_end_matches(".enc").to_string();
                        println!("Using existing wallet as address: {}", first_wallet.green());
                        
                        // Update config
                        if let serde_yaml::Value::Mapping(ref mut map) = config {
                            map.insert(
                                serde_yaml::Value::String("address".to_string()),
                                serde_yaml::Value::String(first_wallet.clone())
                            );
                            save_config(&config).expect("Failed to save configuration");
                        }
                        
                        Ok(first_wallet)
                    },
                    _ => {
                        println!("{}", "No valid wallets found!".red());
                        println!("Creating a new wallet...");
                        create_new_wallet(None)
                    }
                }
            } else {
                Ok(address.to_string())
            }
        },
        None => {
            // Try to find any existing wallet
            match list_wallet_files() {
                Ok(wallets) if !wallets.is_empty() => {
                    let first_wallet = wallets[0].0.trim_end_matches(".enc").to_string();
                    println!("Setting address to existing wallet: {}", first_wallet.green());
                    
                    // Update config with new address
                    if let serde_yaml::Value::Mapping(ref mut map) = config {
                        map.insert(
                            serde_yaml::Value::String("address".to_string()),
                            serde_yaml::Value::String(first_wallet.clone())
                        );
                        save_config(&config).expect("Failed to save configuration");
                    }
                    
                    Ok(first_wallet)
                },
                _ => {
                    println!("{}", "No wallets found!".red());
                    println!("Creating a new wallet...");
                    create_new_wallet(None)
                }
            }
        }
    }
}

/// Create a new wallet and return the address
pub fn create_wallet(password: Option<String>) -> Result<String> {
    create_new_wallet(password)
}

/// Create a new wallet with the specified password
fn create_new_wallet(password: Option<String>) -> Result<String> {
    // Generate a new address - 12 word mnemonic
    let (private_key, public_address, seed_phrase) = generate_karix_address(12);
    
    // Remove the "0x" prefix from the address if present
    let clean_address = public_address.trim_start_matches("0x").to_string();
    
    // Parse the address - use the correct method
    let address = Address::from_hex(&clean_address)
        .map_err(|e| NodeError::ConfigError(format!("Invalid address format: {}", e)))?;
    
    // Use provided password or a default
    let pwd = password.unwrap_or_else(|| "password123".to_string());
    
    // Save the wallet
    save_wallet(&address, &private_key, &seed_phrase, &pwd)
        .map_err(|e| NodeError::WalletError(e))?;
    
    // Update the selected wallet
    set_selected_wallet(&clean_address)
        .map_err(|e| NodeError::IoError(e))?;
    
    println!("{} Created new wallet with address: {}", "Success:".green(), clean_address.green());
    println!("{} {}", "Private Key:".yellow(), private_key);
    println!("{} {}", "Seed Phrase:".yellow(), seed_phrase);
    println!("\n{} {}", "IMPORTANT:".red().bold(), "Store your private key and seed phrase safely!".bold());
    
    Ok(clean_address)
}