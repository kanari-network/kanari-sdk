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

/// Load configuration (now completely from kanari.yaml)
pub fn load_config() -> io::Result<Value> {
    let kanari_config = load_kanari_config()?;
    
    let active_env_str = match kanari_config.get("active_env").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok(Value::Mapping(Mapping::new())), // No active_env, return empty
    };

    let envs = match kanari_config.get("envs").and_then(|v| v.as_sequence()) {
        Some(s) => s,
        None => return Ok(Value::Mapping(Mapping::new())), // No envs sequence, return empty
    };

    if let Some(active_env_config) = envs.iter().find(|env| {
        env.get("alias").and_then(|v| v.as_str()) == Some(active_env_str)
    }) {
        let mut config_map = Mapping::new();

        let chain_id = match active_env_str {
            "local" => "kari-local-001",
            "dev" => "kari-dev-001",
            "test" => "kari-testnet-001",
            "main" => "kari-mainnet-001",
            _ => "kari-testnet-001", // Default or consider error
        };
        config_map.insert(Value::String("chain_id".to_string()), Value::String(chain_id.to_string()));

        if let Some(addr) = kanari_config.get("active_address").and_then(|v| v.as_str()) {
            config_map.insert(Value::String("address".to_string()), Value::String(addr.to_string()));
        }

        if let Some(rpc_url) = active_env_config.get("rpc").and_then(|v| v.as_str()) {
            let rpc_port = if rpc_url.starts_with("http://127.0.0.1:") || rpc_url.starts_with("http://localhost:") {
                rpc_url.split(':').nth(2).and_then(|p_str| p_str.parse::<u64>().ok()).unwrap_or(30030)
            } else {
                30030 // Default for remote or unparseable local
            };
            config_map.insert(Value::String("rpc_port".to_string()), Value::Number(serde_yaml::Number::from(rpc_port)));
        } else {
             config_map.insert(Value::String("rpc_port".to_string()), Value::Number(serde_yaml::Number::from(30030u64))); // Default if rpc field is missing
        }
        
        return Ok(Value::Mapping(config_map));
    }
    
    Ok(Value::Mapping(Mapping::new())) // Active environment not found in envs list
}

/// Save configuration to kanari.yaml file
pub fn save_config(config_to_save: &Value) -> io::Result<()> {
    let mut kanari_config = load_kanari_config().unwrap_or_else(|_| Value::Mapping(Mapping::new()));
    
    let active_env_alias = match kanari_config.get("active_env").and_then(|v| v.as_str()) {
        Some(alias) => alias.to_string(),
        None => return Ok(()), // No active_env to update
    };

    let config_to_save_map = match config_to_save.as_mapping() {
        Some(map) => map,
        None => return Ok(()), // Nothing to save if not a mapping
    };

    if let Some(kanari_config_map) = kanari_config.as_mapping_mut() {
        // Update active_address if "address" is in config_to_save
        if let Some(addr_val) = config_to_save_map.get("address").and_then(|v| v.as_str()) {
            kanari_config_map.insert(
                Value::String("active_address".to_string()),
                Value::String(addr_val.to_string()),
            );
        }

        // Update RPC URL in the active environment if "rpc_port" is in config_to_save
        if let Some(envs) = kanari_config_map.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
            if let Some(env_to_update) = envs.iter_mut().find(|env| {
                env.get("alias").and_then(|v| v.as_str()) == Some(&active_env_alias)
            }) {
                if let Some(rpc_port_val) = config_to_save_map.get("rpc_port").and_then(|v| v.as_u64()) {
                    if let Some(env_map_mut) = env_to_update.as_mapping_mut() {
                        env_map_mut.insert(
                            Value::String("rpc".to_string()),
                            Value::String(format!("http://127.0.0.1:{}", rpc_port_val)),
                        );
                    }
                }
            }
        }
        
        save_kanari_config(&Value::Mapping(kanari_config_map.clone()))?; // Clone because save_kanari_config takes &Value
    }
    
    Ok(())
}

/// Get current main wallet address
pub fn get_main_wallet() -> Option<String> {
    load_kanari_config().ok()
        .and_then(|config| config.get("active_address")
            .and_then(|v| v.as_str())
            .map(String::from))
}
