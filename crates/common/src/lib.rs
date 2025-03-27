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

/// Load configuration from file with address formatting
pub fn load_config() -> io::Result<Value> {
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
    let config_path = get_kari_dir().join("config.yaml");
    let mut file = File::create(config_path)?;
    
    // Create a copy for modification
    let mut config = config.clone();
    
    // Clean up address format before saving
    if let Some(mapping) = config.as_mapping_mut() {
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
    Ok(())
}
