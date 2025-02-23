use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]  // Add PartialEq
pub enum NetworkType {
    Mainnet,
    Testnet,
    Devnet,
}


#[derive(Serialize, Deserialize)]
pub struct NetworkConfig {
    pub node_address: String,
    pub domain: String,     // Add this field
    pub port: u16,
    pub peers: Vec<String>,
    pub chain_id: String,
    pub max_connections: u32,
    pub api_enabled: bool,
    pub network_type: NetworkType,
}


impl fmt::Display for NetworkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkType::Mainnet => write!(f, "mainnet"),
            NetworkType::Testnet => write!(f, "testnet"),
            NetworkType::Devnet => write!(f, "devnet"),
        }
    }
}

impl NetworkConfig {
    // Loads configuration from a specified file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: NetworkConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    // Saves the current configuration to a specified file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    // Updates the configuration with new values
    pub fn update(&mut self, new_config: NetworkConfig) {
        self.node_address = new_config.node_address;
        self.port = new_config.port;
        self.peers = new_config.peers;
        self.chain_id = new_config.chain_id;
        self.max_connections = new_config.max_connections;
        self.api_enabled = new_config.api_enabled;
        self.network_type = new_config.network_type;
    }
}