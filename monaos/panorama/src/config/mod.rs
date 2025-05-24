// src/config.rs
use std::io::{self, Write};
use serde_yaml::{Value, Mapping};
use network::NetworkConfig;

// Simply re-export functions from common
use common::{load_config, save_config, load_kanari_config, save_kanari_config};


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
        
        // Check for security settings and add defaults if missing
        let localhost_only = mapping.get("localhost_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let use_tls = mapping.get("use_tls")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let trusted_peers = mapping.get("trusted_peers")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                   .filter_map(|v| v.as_str())
                   .map(|s| s.to_string())
                   .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| Vec::new());
        
        return Ok(NetworkConfig {
            node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
            port: mapping.get("rpc_port").and_then(|v| v.as_u64()).unwrap_or(30030) as u16,
            peers: vec![],
            chain_id: mapping.get("chain_id").and_then(|v| v.as_str()).unwrap_or(chain_id).to_string(),
            max_connections: 100,
            api_enabled: true,
            localhost_only,
            use_tls,
            trusted_peers,
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
    
    // Prompt for security settings
    let localhost_only_str = prompt_for_value("Restrict to localhost only? (true/false)", "false");
    let localhost_only = localhost_only_str.to_lowercase() == "true";
    mapping.insert(
        Value::String("localhost_only".to_string()),
        Value::Bool(localhost_only)
    );
    
    let use_tls_str = prompt_for_value("Use TLS encryption for P2P communication? (true/false)", "false");
    let use_tls = use_tls_str.to_lowercase() == "true";
    mapping.insert(
        Value::String("use_tls".to_string()),
        Value::Bool(use_tls)
    );
    
    // Add empty trusted peers list
    let trusted_peers = Vec::<String>::new();
    mapping.insert(
        Value::String("trusted_peers".to_string()),
        Value::Sequence(trusted_peers.iter().map(|s| Value::String(s.clone())).collect())
    );

    // Update the kanari.yaml configuration as well
    let mut kanari_config = load_kanari_config()?;
    if let Some(kanari_mapping) = kanari_config.as_mapping_mut() {
        let active_env = kanari_mapping.get("active_env")
                         .and_then(|v| v.as_str())
                         .unwrap_or("local")
                         .to_string();
                         
        if let Some(envs) = kanari_mapping.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
            for env in envs {
                if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                    if alias == active_env {
                        // Update RPC URL with the new port
                        env["rpc"] = Value::String(format!("http://127.0.0.1:{}", rpc_port));
                        break;
                    }
                }
            }
        }
        
        // Save the updated kanari config
        save_kanari_config(&kanari_config)?;
    }

    let network_config = NetworkConfig {
        node_address: get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string()),
        port: rpc_port,
        peers: vec![],
        chain_id: chain_id.to_string(),
        max_connections: 100,
        api_enabled: true,
        localhost_only,
        use_tls,
        trusted_peers,
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