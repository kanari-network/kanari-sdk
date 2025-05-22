use dirs;
use std::fs::{self, File};
use std::path::PathBuf;
use serde_yaml::{Value, Mapping};
use std::io::{self, Write};


// Path utility functions
pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

/// Function to format address by removing .enc suffix if present
fn format_address(addr: &str) -> String {
    addr.trim_end_matches(".enc").to_string()
}

/// Get path to the kanari.yaml configuration file
pub fn get_kanari_config_path() -> PathBuf {
    let mut config_dir = get_kari_dir();
    config_dir.push("kanari_config");
    fs::create_dir_all(&config_dir).expect("Unable to create kanari_config directory");
    config_dir.push("kanari.yaml");
    config_dir
}

/// Load configuration from kanari.yaml file
pub fn load_kanari_config() -> io::Result<Value> {
    let config_path = get_kanari_config_path();
    
    // Return empty config if file doesn't exist
    if !config_path.exists() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    
    // Read and parse config file
    let config_str = fs::read_to_string(&config_path)?;
    
    // Return empty config if file is empty
    if config_str.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    
    // Parse YAML with error handling
    let config: Value = serde_yaml::from_str(&config_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData, 
            format!("Failed to parse kanari.yaml file: {}", e)
        )
    })?;
    
    Ok(config)
}

/// Save configuration to kanari.yaml file
pub fn save_kanari_config(config: &Value) -> io::Result<()> {
    let config_path = get_kanari_config_path();
    let mut file = File::create(config_path)?;
    
    // Serialize and save with error handling
    let yaml_str = serde_yaml::to_string(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize config: {}", e)
        )
    })?;
    
    file.write_all(yaml_str.as_bytes())?;
    Ok(())
}

// Legacy functions - maintained for backward compatibility
pub fn load_config() -> io::Result<Value> {
    // First try to get configuration from kanari.yaml
    if let Ok(kanari_config) = load_kanari_config() {
        if let Some(active_env) = kanari_config.get("active_env").and_then(|v| v.as_str()) {
            if let Some(envs) = kanari_config.get("envs").and_then(|v| v.as_sequence()) {
                // Find the active environment
                for env in envs {
                    if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                        if alias == active_env {
                            // Create a config-compatible structure
                            let mut config = Mapping::new();
                            
                            // Add chain_id based on environment
                            let chain_id = match active_env {
                                "local" => "kari-local-001",
                                "dev" => "kari-dev-001",
                                "test" => "kari-testnet-001",
                                "main" => "kari-mainnet-001",
                                _ => "kari-testnet-001"
                            };
                            
                            config.insert(
                                Value::String("chain_id".to_string()),
                                Value::String(chain_id.to_string())
                            );
                            
                            // Add address from kanari config
                            if let Some(addr) = kanari_config.get("active_address").and_then(|v| v.as_str()) {
                                config.insert(
                                    Value::String("address".to_string()),
                                    Value::String(format_address(addr))
                                );
                            }
                            
                            // Add RPC port from the URL
                            if let Some(rpc_url) = env.get("rpc").and_then(|v| v.as_str()) {
                                if rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1") {
                                    // Extract port from local URL
                                    if let Some(port_str) = rpc_url.split(':').nth(2) {
                                        if let Ok(port) = port_str.parse::<u64>() {
                                            config.insert(
                                                Value::String("rpc_port".to_string()),
                                                Value::Number(serde_yaml::Number::from(port))
                                            );
                                        }
                                    } else {
                                        // Default port if not specified
                                        config.insert(
                                            Value::String("rpc_port".to_string()),
                                            Value::Number(serde_yaml::Number::from(30030u64))
                                        );
                                    }
                                } else {
                                    // Remote server, just default to 30030 for local RPC
                                    config.insert(
                                        Value::String("rpc_port".to_string()),
                                        Value::Number(serde_yaml::Number::from(30030u64))
                                    );
                                }
                            }
                            
                            return Ok(Value::Mapping(config));
                        }
                    }
                }
            }
        }
    }
    
    // Fall back to the old config.yaml if kanari.yaml processing failed
    let config_path = get_kari_dir().join("config.yaml");
    
    // Return empty config if file doesn't exist
    if !config_path.exists() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    
    // Read and parse config file
    let config_str = fs::read_to_string(&config_path)?;
    
    // Return empty config if file is empty
    if config_str.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    
    // Parse YAML with error handling
    let mut config: Value = serde_yaml::from_str(&config_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData, 
            format!("Failed to parse config file: {}", e)
        )
    })?;

    // Clean up address format
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

/// Save configuration to file with address formatting
pub fn save_config(config: &Value) -> io::Result<()> {
    // Get current kanari config
    let mut kanari_config = load_kanari_config().unwrap_or_else(|_| Value::Mapping(Mapping::new()));
    
    // If we have an active_env, update that environment
    // Clone the active_env to end the immutable borrow
    let active_env = if let Some(env) = kanari_config.get("active_env").and_then(|v| v.as_str()) {
        env.to_string()
    } else {
        return Ok(());
    };

    if let Some(mapping) = config.as_mapping() {
        // Create or update the environment
        if let Some(envs) = kanari_config.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
                // Find and update the active environment
                for env in envs {
                    if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                        if alias == active_env {
                            // Update the RPC port if it exists in the config
                            if let Some(rpc_port) = mapping.get("rpc_port") {
                                if let Some(port) = rpc_port.as_u64() {
                                    env["rpc"] = Value::String(format!("http://127.0.0.1:{}", port));
                                }
                            }
                            break;
                        }
                    }
                }
            }
            
            // Update active_address if address exists in the config
            if let Some(addr) = mapping.get("address").and_then(|v| v.as_str()) {
                if let Some(mapping) = kanari_config.as_mapping_mut() {
                    mapping.insert(
                        Value::String("active_address".to_string()),
                        Value::String(format_address(addr))
                    );
                }
            }
            
            // Save the updated kanari config
            save_kanari_config(&kanari_config)?;
    } else {
        // Fall back to saving the old config.yaml
        let config_path = get_kari_dir().join("config.yaml");
        let mut file = File::create(config_path)?;
        
        // Create a copy for modification
        let mut config = config.clone();
        
        // Clean up address format before saving
        if let Some(mapping) = config.as_mapping_mut() {
            // Format wallet address
            if let Some(addr) = mapping.get("address").and_then(|v| v.as_str()) {
                mapping.insert(
                    Value::String("address".to_string()),
                    Value::String(format_address(addr))
                );
            }
        }
        
        // Serialize and save with error handling
        let yaml_str = serde_yaml::to_string(&config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize config: {}", e)
            )
        })?;
        
        file.write_all(yaml_str.as_bytes())?;
    }
    
    Ok(())
}

/// Get current main wallet address
pub fn get_main_wallet() -> Option<String> {
    // First try kanari config
    if let Ok(kanari_config) = load_kanari_config() {
        if let Some(addr) = kanari_config.get("active_address").and_then(|v| v.as_str()) {
            return Some(addr.to_string());
        }
    }
    
    // Fall back to old config
    match load_config() {
        Ok(config) => {
            config.get("address").and_then(|v| v.as_str()).map(|s| s.to_string())
        },
        Err(_) => None
    }
}
