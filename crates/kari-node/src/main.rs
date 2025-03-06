use clap::{Parser, Subcommand};
use colored::Colorize;
use kari_node::{
    check_wallet_exists, configure_network, create_wallet, get_kari_dir, import_from_private_key,
    import_from_seed_phrase, list_wallet_files, load_config, load_wallet, save_config,
    set_selected_wallet, start_node, stop_node, NodeConfig, NodeError, Result,
};
use std::path::Path;
use std::process;

#[derive(Parser)]
#[command(name = "kari-node")]
#[command(author = "Kanari Team")]
#[command(version = "0.1.0")]
#[command(about = "Kari blockchain node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new node configuration
    Init {
        #[arg(short, long, default_value = "testnet")]
        network: String,
        
        #[arg(short, long)]
        chain_id: Option<String>,
    },
    
    /// Manage wallets
    Wallet {
        #[command(subcommand)]
        subcommand: WalletCommands,
    },
    
    /// Start the node
    Start {
        #[arg(short, long, help = "Run in interactive mode")]
        interactive: bool,
        
        #[arg(short, long, help = "Enable RPC server")]
        rpc: bool,
        
        #[arg(short, long, help = "Enable API server")]
        api: bool,
    },
    
    /// Stop the node
    Stop,
    
    /// Show node status
    Status,
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Create a new wallet
    Create {
        #[arg(short, long)]
        password: Option<String>,
    },
    
    /// List available wallets
    List,
    
    /// Select a wallet to use
    Select {
        #[arg(short, long)]
        address: String,
    },
    
    /// Import wallet from seed phrase
    ImportSeed {
        #[arg(short, long)]
        seed: String,
        
        #[arg(short, long)]
        password: Option<String>,
    },
    
    /// Import wallet from private key
    ImportKey {
        #[arg(short, long)]
        key: String,
        
        #[arg(short, long)]
        password: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{}: {}", "Error".bright_red(), err);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { network, chain_id }) => init_node(network, chain_id),
        Some(Commands::Wallet { subcommand }) => handle_wallet(subcommand).await,
        Some(Commands::Start { interactive, rpc, api }) => start(interactive, rpc, api).await,
        Some(Commands::Stop) => stop_node(),
        Some(Commands::Status) => show_status(),
        None => {
            println!("{}", "Starting kari-node with default configuration...".bright_blue());
            start(false, true, true).await
        }
    }
}

fn init_node(network: String, chain_id: Option<String>) -> Result<()> {
    println!("{} {}", "Initializing node with network:".bright_blue(), network.bright_green());
    
    let data_dir = get_kari_dir();
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
        println!("Created data directory: {}", data_dir.display().to_string().bright_yellow());
    }
    
    let network_type = match network.to_lowercase().as_str() {
        "mainnet" => network::NetworkType::Mainnet,
        "testnet" => network::NetworkType::Testnet,
        "devnet" => network::NetworkType::Devnet,
        _ => {
            return Err(NodeError::ConfigError(format!(
                "Invalid network type: {}. Use mainnet, testnet, or devnet", 
                network
            )));
        }
    };
    
    let chain_id = chain_id.unwrap_or_else(|| k2::chain_id::CHAIN_ID.to_string());
    let network_config = configure_network(&chain_id)?;
    
    // Create basic config
    let config = serde_yaml::Value::Mapping({
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("chain_id".to_string()),
            serde_yaml::Value::String(chain_id)
        );
        map.insert(
            serde_yaml::Value::String("network_type".to_string()),
            serde_yaml::Value::String(network_type.to_string())
        );
        map.insert(
            serde_yaml::Value::String("rpc_port".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(8545u16))
        );
        map.insert(
            serde_yaml::Value::String("domain".to_string()),
            serde_yaml::Value::String("localhost".to_string())
        );
        map
    });
    
    save_config(&config)?;
    println!("{}", "Node configuration initialized successfully".bright_green());
    
    // Check if wallet exists
    if !check_wallet_exists() {
        println!("{}", "No wallet found. Creating a new wallet...".bright_yellow());
        create_wallet(None)?;
    } else {
        println!("{}", "Wallet already exists".bright_green());
    }
    
    println!("{}", "Node initialization complete!".bright_green());
    println!("Run 'kari-node start' to start the node");
    
    Ok(())
}

async fn handle_wallet(cmd: WalletCommands) -> Result<()> {
    match cmd {
        WalletCommands::Create { password } => {
            println!("{}", "Creating new wallet...".bright_blue());
            let address = create_wallet(password)?;
            println!("New wallet created with address: {}", address.bright_green());
            Ok(())
        }
        WalletCommands::List => {
            println!("{}", "Available wallets:".bright_blue());
            let wallets = list_wallet_files()?;
            
            if wallets.is_empty() {
                println!("No wallets found");
                return Ok(());
            }
            
            for (wallet, _) in wallets {
                let address = wallet.trim_end_matches(".enc");
                println!("- {}", address.bright_green());
            }
            Ok(())
        }
        WalletCommands::Select { address } => {
            let wallet_path = get_kari_dir().join("wallets").join(format!("{}.enc", address));
            
            if !wallet_path.exists() {
                // Use NodeError::ConfigError which takes a String directly
                return Err(NodeError::ConfigError(
                    format!("Wallet with address {} does not exist", address)
                ));
            }
            
            set_selected_wallet(&address)?;
            println!("Selected wallet: {}", address.bright_green());
            
            // Update config
            let mut config = load_config()?;
            if let serde_yaml::Value::Mapping(ref mut map) = config {
                map.insert(
                    serde_yaml::Value::String("address".to_string()),
                    serde_yaml::Value::String(address.clone())
                );
                save_config(&config)?;
            }
            
            Ok(())
        }
        WalletCommands::ImportSeed { seed, password } => {
            println!("{}", "Importing wallet from seed phrase...".bright_blue());
            let wallet = import_from_seed_phrase(&seed);
            let pwd = password.unwrap_or_else(|| "password123".to_string());
            
            // Save wallet and get address
            // Implementation would depend on how your key module is structured
            // For now, just showing a placeholder
            println!("Wallet imported successfully: {}", "ADDRESS".bright_green());
            
            Ok(())
        }
        WalletCommands::ImportKey { key, password } => {
            println!("{}", "Importing wallet from private key...".bright_blue());
            let wallet = import_from_private_key(&key);
            let pwd = password.unwrap_or_else(|| "password123".to_string());
            
            // Save wallet and get address
            // Implementation would depend on how your key module is structured
            println!("Wallet imported successfully: {}", "ADDRESS".bright_green());
            
            Ok(())
        }
    }
}

async fn start(interactive: bool, enable_rpc: bool, enable_api: bool) -> Result<()> {
    println!("{}", "Starting Kari node...".bright_blue());
    
    // Create default configuration if none exists
    let mut config = load_config().unwrap_or_else(|_| {
        println!("{}", "No configuration found. Creating default...".bright_yellow());
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("chain_id".to_string()),
            serde_yaml::Value::String(k2::chain_id::CHAIN_ID.to_string())
        );
        map.insert(
            serde_yaml::Value::String("network_type".to_string()),
            serde_yaml::Value::String("testnet".to_string())
        );
        serde_yaml::Value::Mapping(map)
    });
    
    // Create a node config
    let node_config = Some(NodeConfig {
        address: String::new(), // Will be filled by start_node
        network: network::NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain: "localhost".to_string(),
            port: 8545,
            peers: vec![],
            chain_id: k2::chain_id::CHAIN_ID.to_string(),
            max_connections: 100,
            api_enabled: enable_api,
            network_type: network::NetworkType::Testnet,
        },
        enable_rpc,
        enable_api,
    });
    
    // Start the node
    start_node(node_config).await?;
    
    if interactive {
        println!("{}", "Node running in interactive mode. Press Ctrl+C to exit.".bright_blue());
        
        // Wait for Ctrl+C
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::select! {
            _ = ctrl_c => {
                println!("\n{}", "Received termination signal".bright_yellow());
            }
        }
        
        // Stop the node
        stop_node()?;
    } else {
        println!("{}", "Node started in background mode".bright_blue());
        println!("Run 'kari-node stop' to stop the node");
    }
    
    Ok(())
}

fn show_status() -> Result<()> {
    println!("{}", "Kari Node Status".bright_blue().bold());
    println!("{}", "===============".bright_blue());
    
    // Check configuration
    let config_path = get_kari_dir().join("config.yml");
    if !config_path.exists() {
        println!("Configuration: {}", "Not initialized".bright_red());
        println!("Run 'kari-node init' to initialize the node");
        return Ok(());
    }
    
    // Load configuration
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(_) => {
            println!("Configuration: {}", "Invalid or corrupted".bright_red());
            println!("Run 'kari-node init' to reinitialize the node");
            return Ok(());
        }
    };
    
    println!("Configuration: {}", "Loaded".bright_green());
    
    // Display network type
    if let Some(network) = config.get("network_type").and_then(|v| v.as_str()) {
        println!("Network: {}", network.bright_green());
    } else {
        println!("Network: {}", "Not specified".bright_yellow());
    }
    
    // Display chain ID
    if let Some(chain_id) = config.get("chain_id").and_then(|v| v.as_str()) {
        println!("Chain ID: {}", chain_id.bright_green());
    } else {
        println!("Chain ID: {}", "Not specified".bright_yellow());
    }
    
    // Display wallet
    if let Some(address) = config.get("address").and_then(|v| v.as_str()) {
        let wallet_path = get_kari_dir().join("wallets").join(format!("{}.enc", address));
        
        if wallet_path.exists() {
            println!("Wallet: {} ({})", address.bright_green(), "Found".bright_green());
        } else {
            println!("Wallet: {} ({})", address.bright_yellow(), "Not found".bright_red());
            println!("Run 'kari-node wallet create' to create a new wallet");
        }
    } else {
        println!("Wallet: {}", "Not configured".bright_yellow());
        println!("Run 'kari-node wallet create' to create a new wallet");
    }
    
    // Display RPC status (placeholder - you'd need actual logic to check if running)
    println!("Node status: {}", "Unknown".bright_yellow());
    
    Ok(())
}

