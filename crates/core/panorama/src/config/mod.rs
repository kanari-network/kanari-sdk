// src/config.rs
use std::io::{self, Write};
use std::path::PathBuf;
use serde_yaml::{Value, Mapping};
use std::fs::{self, File};
use dirs;
use network::{NetworkConfig, NetworkType};


// Function to get the configuration directory
pub fn get_config_dir() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;
    Ok(config_path)
}

pub fn format_address(address: &str) -> String {
    address.trim_end_matches(".enc").to_string()
}

// Function to load the configuration from file
pub fn load_config() -> io::Result<Value> {
    let config_file_path = get_config_dir()?.join("config.yaml");
    
    // If file doesn't exist, return empty YAML object
    if !config_file_path.exists() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }
    
    // Read the file content
    let config_str = fs::read_to_string(&config_file_path)?;
    
    // If the file is empty, return empty YAML object
    if config_str.trim().is_empty() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }
    
    // Parse YAML and clean up address format
    let mut config: Value = serde_yaml::from_str(&config_str).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse config file: {}", e))
    })?;

    // Remove .enc suffix from address if present
    if let Some(mapping) = config.as_mapping_mut() {
        if let Some(addr) = mapping.get("address").and_then(|v| v.as_str()) {
            mapping.insert(
                Value::String("address".to_string()),
                Value::String(format_address(addr))
            );
        }
    }
    
    Ok(config)
}

// Function to save the configuration to file
pub fn save_config(config: &Value) -> Result<(), std::io::Error> {
    let config_dir = get_config_dir()?;
    let config_file_path = config_dir.join("config.yaml");
    let mut file = File::create(config_file_path)?;
    
    // Clean up any .enc suffixes before saving
    let mut config = config.clone();
    if let Some(mapping) = config.as_mapping_mut() {
        if let Some(addr) = mapping.get("address").and_then(|v| v.as_str()) {
            mapping.insert(
                Value::String("address".to_string()),
                Value::String(format_address(addr))
            );
        }
    }
    
    let yaml_str = serde_yaml::to_string(&config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    file.write_all(yaml_str.as_bytes())?;
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


// Function to configure the network settings
pub fn configure_network(chain_id: &str) -> io::Result<NetworkConfig> {
    let mut config = load_config()?;
    let mut default_mapping = Mapping::new();
    let mapping = config.as_mapping_mut().unwrap_or(&mut default_mapping);

    // Check if configuration exists
    if mapping.contains_key("network_type") && 
       mapping.contains_key("rpc_port") && 
       mapping.contains_key("domain") && 
       mapping.contains_key("chain_id") {
        println!("Configuration already exists. Skipping configuration process.");
        let network_type = match mapping.get("network_type").and_then(|v| v.as_str()) {
            Some("devnet") => NetworkType::Devnet,
            Some("testnet") => NetworkType::Testnet,
            Some("mainnet") => NetworkType::Mainnet,
            _ => NetworkType::Mainnet,
        };

        return Ok(NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain: mapping.get("domain").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            port: mapping.get("rpc_port").and_then(|v| v.as_u64()).unwrap_or(30030) as u16,
            peers: vec![],
            chain_id: mapping.get("chain_id").and_then(|v| v.as_str()).unwrap_or(chain_id).to_string(),
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

    // Update configuration with YAML values
    mapping.insert(
        Value::String("network_type".to_string()),
        Value::String(network_type_input.to_string())
    );
    
    let default_rpc_port = match network_type_input {
        "devnet" => "30031",
        "testnet" => "30032",
        "mainnet" => "30030",
        _ => "30030", // Default to mainnet port
    };

    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    mapping.insert(
        Value::String("rpc_port".to_string()),
        Value::Number(rpc_port.into())
    );

    let default_domain = match network_type_input {
        "devnet" => "devnet.kanari.network",
        "testnet" => "testnet.kanari.network",
        "mainnet" => "mainnet.kanari.network",
        _ => "mainnet.kanari.network", // Default to mainnet domain
    };
    
    // Prompt the user for the network domain
    let domain = prompt_for_value("Enter network domain", default_domain);
    mapping.insert(
        Value::String("domain".to_string()),
        Value::String(domain.clone())
    );

    mapping.insert(
        Value::String("chain_id".to_string()),
        Value::String(chain_id.to_string())
    );

    // Save the configuration to file
    let network_config = NetworkConfig {
        node_address: "127.0.0.1".to_string(),
        domain: domain,         // Add configured domain
        port: rpc_port, // Use the parsed rpc_port
        peers: vec![],
        chain_id: chain_id.to_string(),
        max_connections: 100,
        api_enabled: true,
        network_type,
    };

    // Create owned Mapping from mutable reference
    let owned_mapping = mapping.clone();
    
    // Save configuration with owned Mapping
    save_config(&Value::Mapping(owned_mapping))?;

    println!("Network configuration saved successfully.");
    Ok(network_config) // Return the NetworkConfig
}