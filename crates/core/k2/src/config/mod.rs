// src/config.rs
use dirs;
use network::{NetworkConfig, NetworkType};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
// use std::net::{TcpStream, ToSocketAddrs};
// use std::time::Duration;


/// Returns the platform-specific configuration directory
pub fn get_config_dir() -> io::Result<PathBuf> {
    dirs::config_dir()
        .or_else(|| dirs::home_dir())
        .map(|dir| dir.join(".kari").join("network"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine configuration directory",
            )
        })
        .and_then(|path| {
            fs::create_dir_all(&path)?;
            Ok(path)
        })
}

// Function to load the configuration from file
pub fn load_config() -> io::Result<Value> {
    let config_file_path = get_config_dir()?.join("config.json");

    // If file doesn't exist, return empty JSON object
    if !config_file_path.exists() {
        return Ok(json!({}));
    }

    // Read the file content
    let config_str = fs::read_to_string(&config_file_path)?;

    // If the file is empty, return empty JSON object
    if config_str.trim().is_empty() {
        return Ok(json!({}));
    }

    // Parse the JSON, handle parsing errors explicitly
    match serde_json::from_str(&config_str) {
        Ok(json) => Ok(json),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse config file: {}", e),
        )),
    }
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
    loop {
        print!("{} [{}]: ", prompt, default);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed_input = input.trim();
        if trimmed_input.is_empty() {
            return default.to_string();
        } else {
            if prompt == "Enter RPC port" {
                if trimmed_input.parse::<u16>().is_ok() {
                    return trimmed_input.to_string();
                } else {
                    println!("Invalid port number. Please enter a valid u16 value.");
                    continue;
                }
            }
            return trimmed_input.to_string();
        }
    }
}


// Add domain constants at the top of the file
const BASE_DOMAIN: &str = "kanari.network";
const NETWORK_DOMAINS: [(NetworkType, &str); 3] = [
    (NetworkType::Devnet, "devnet"),
    (NetworkType::Testnet, "testnet"),
    (NetworkType::Mainnet, "mainnet"),
];

// Add helper function to get domain
fn get_network_domain(network_type: &NetworkType) -> String {
    let subdomain = NETWORK_DOMAINS
        .iter()
        .find(|(net_type, _)| *net_type == *network_type)  // Dereference for comparison
        .map(|(_, subdomain)| *subdomain)  // Dereference subdomain
        .unwrap_or("mainnet");
    format!("{}.{}", subdomain, BASE_DOMAIN)
}

// Function to configure the network settings
pub fn configure_network(chain_id: &str) -> io::Result<NetworkConfig> {
    let mut config = load_config()?;

    // Check if the configuration already exists
    if config.get("network_type").is_some() {
        
        println!("Configuration already exists. Skipping configuration process.");
        let network_type = match config.get("network_type").unwrap().as_str().unwrap() {
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            "mainnet" => NetworkType::Mainnet,
            _ => NetworkType::Mainnet,
        };

        let chain_id = config
            .get("chain_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let node_address = config
            .get("node_address")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1")
            .to_string();

        let domain = config
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("mainnet.kanari.network")
            .to_string();

        return Ok(NetworkConfig {
            node_address,
            domain, // Add domain
            port: config.get("rpc_port").unwrap().as_u64().unwrap() as u16,
            peers: vec![],
            chain_id: chain_id.to_string(),
            max_connections: 100,
            api_enabled: true,
            network_type,
        });
    }

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

    config
        .as_object_mut()
        .unwrap()
        .insert("network_type".to_string(), json!(network_type_input));

    let default_rpc_port = match network_type_input {
        "devnet" => "3031",
        "testnet" => "3032",
        "mainnet" => "3030",
        _ => "3030", // Default to mainnet port
    };

    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    config
        .as_object_mut()
        .unwrap()
        .insert("rpc_port".to_string(), json!(rpc_port));

    // Prompt for node address
    let default_node_address = "127.0.0.1";
    // Get the node address from the user
    let node_address = prompt_for_value("Enter node address", default_node_address);
    // Save the chain ID to the configuration
    config
        .as_object_mut()
        .unwrap()
        .insert("chain_id".to_string(), json!(chain_id));

    let domain = get_network_domain(&network_type);
    config
        .as_object_mut()
        .unwrap()
        .insert("domain".to_string(), json!(domain));

    // Save the configuration to file
    let network_config = NetworkConfig {
        node_address,
        domain,
        port: rpc_port,
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
