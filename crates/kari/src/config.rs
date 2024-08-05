// src/config.rs
use std::io::{self, Write};
use std::path::PathBuf;
use serde_json::{json, Value};
use std::fs::{self, File};
use dirs;
use consensus_core::{NetworkConfig, NetworkType};

// Function to get the configuration directory
pub fn get_config_dir() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;
    Ok(config_path)
}

// Function to load the configuration from file
pub fn load_config() -> io::Result<Value> {
    let config_file_path = get_config_dir()?.join("config.json");
    if !config_file_path.exists() {
        return Ok(json!({})); // Return an empty JSON object if the file doesn't exist
    }
    let config_str = fs::read_to_string(config_file_path)?;
    Ok(serde_json::from_str(&config_str)?)
}

// Function to save the configuration to file
pub fn save_config(config: &Value) -> Result<(), std::io::Error> {
    let config_dir = get_config_dir()?;
    let config_file_path = config_dir.join("config.json");
    let mut file = File::create(config_file_path)?;
    file.write_all(config.to_string().as_bytes())?;
    Ok(())
}

// Function to prompt the user for a value with a default
pub fn prompt_for_value(prompt: &str, default: &str) -> String {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        default.to_string()
    } else {
        trimmed_input.to_string()
    }
}

// Function to configure the network settings
pub fn configure_network(chain_id: &str) -> io::Result<NetworkConfig> {
    let mut config = load_config()?;

    println!("Choose a network type:");
    println!("1. devnet");
    println!("2. testnet");
    println!("3. mainnet");

    let network_type_input = loop {
        print!("Enter your choice [1-3]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().parse::<u32>() {
            Ok(1) => break "devnet",
            Ok(2) => break "testnet",
            Ok(3) => break "mainnet",
            _ => println!("Invalid choice. Please enter a number between 1 and 3."),
        }
    };

    let network_type = match network_type_input {
        "devnet" => NetworkType::Devnet,
        "testnet" => NetworkType::Testnet,
        "mainnet" => NetworkType::Mainnet,
        _ => unreachable!(), // We already validated the input
    };

    config.as_object_mut().unwrap().insert("network_type".to_string(), json!(network_type_input));

    let default_rpc_port = match network_type_input {
        "devnet" => "3031",
        "testnet" => "3032",
        "mainnet" => "3030",
        _ => "3030", // Default to mainnet port
    };
    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    config.as_object_mut().unwrap().insert("rpc_port".to_string(), json!(rpc_port));

    let default_domain = match network_type_input {
        "devnet" => "devnet.kari.network",
        "testnet" => "testnet.kari.network",
        "mainnet" => "kari.network",
        _ => "kari.network", // Default to mainnet domain
    };
    let domain = prompt_for_value("Enter network domain", default_domain);
    config.as_object_mut().unwrap().insert("domain".to_string(), json!(domain));

    // Save the chain ID to the configuration
    config.as_object_mut().unwrap().insert("chain_id".to_string(), json!(chain_id));

    let network_config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        port: rpc_port, // Use the parsed rpc_port
        peers: vec![],
        chain_id: chain_id.to_string(),
        max_connections: 100,
        api_enabled: true,
        network_type,
    };

    save_config(&config)?;

    println!("Network configuration saved successfully.");
    Ok(network_config) // Return the NetworkConfig
}
