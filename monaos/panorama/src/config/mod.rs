// src/config.rs
use std::io::{self, Write};
use serde_yaml::{Value, Mapping};
use network::NetworkConfig;

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
    if mapping.contains_key("rpc_port") &&
       mapping.contains_key("chain_id") {
        println!("Configuration already exists. Skipping configuration process.");
        return Ok(NetworkConfig {
            node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
            port: mapping.get("rpc_port").and_then(|v| v.as_u64()).unwrap_or(30030) as u16,
            peers: vec![],
            chain_id: mapping.get("chain_id").and_then(|v| v.as_str()).unwrap_or(chain_id).to_string(),
            max_connections: 100,
            api_enabled: true,
            // Use domain_peer for P2P connections (bootstrap nodes)
            domain_peer: mapping.get("domain_peer").and_then(|v| v.as_str()).map(String::from)
                .or_else(|| mapping.get("domain").and_then(|v| v.as_str()).map(String::from)), 
            // Use domain for RPC API connections
            domain_api: mapping.get("domain").and_then(|v| v.as_str()).map(String::from), 
            use_tls: mapping.get("use_tls").and_then(|v| v.as_bool()).unwrap_or(false),
        });
    }

    let default_rpc_port = "30030";
    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    mapping.insert(
        Value::String("rpc_port".to_string()),
        Value::Number(rpc_port.into())
    );

    mapping.insert(
        Value::String("chain_id".to_string()),
        Value::String(chain_id.to_string())
    );

    let network_config = NetworkConfig {
        node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
        port: rpc_port,
        peers: vec![],
        chain_id: chain_id.to_string(),
        max_connections: 100,
        api_enabled: true,
        // Initialize domain_peer and domain differently for new configurations
        domain_peer: None, 
        domain_api: None,
        use_tls: false,
    };

    let owned_mapping = mapping.clone();    
    save_config(&Value::Mapping(owned_mapping))?;

    println!("Network configuration saved successfully.");
    Ok(network_config)
}

// Function to get the local IP address
fn get_local_ip() -> Option<String> {
    // Use the improved implementation from node module
    // to avoid code duplication and maintain consistency
    crate::node::get_local_ip()
}