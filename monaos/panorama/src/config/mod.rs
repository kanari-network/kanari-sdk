// src/config.rs
use std::io::{self, Write};
use serde_yaml::{Value, Mapping};
use network::{NetworkConfig, NetworkType};

// Simply re-export functions from common
pub use common::{load_config, save_config};


pub fn format_address(address: &str) -> String {
    address.trim_end_matches(".enc").to_string()
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
            node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
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
    
    // Check if domain is a valid format (either a full domain or *.kanari.network)
    let validated_domain = if domain.contains('.') {
        domain.clone()
    } else {
        // If just a subdomain prefix was entered, append .kanari.network
        format!("{}.kanari.network", domain)
    };
    
    println!("Using domain: {}", validated_domain);
    mapping.insert(
        Value::String("domain".to_string()),
        Value::String(validated_domain.clone())
    );

    mapping.insert(
        Value::String("chain_id".to_string()),
        Value::String(chain_id.to_string())
    );

    // Save the configuration to file
    let network_config = NetworkConfig {
        node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
        domain: validated_domain,         // Use validated domain name
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

// Function to get the local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            if let Ok(_) = socket.connect("8.8.8.8:80") {
                if let Ok(addr) = socket.local_addr() {
                    return Some(addr.ip().to_string());
                }
            }
        }
        Err(_) => {}
    }
    None
}