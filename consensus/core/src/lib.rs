use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct NetworkConfig {
    pub node_address: String,
    pub port: u16,
    pub peers: Vec<String>,
    pub chain_id: String,
    pub max_connections: u32, // New field for maximum connections
    pub api_enabled: bool,    // New field to toggle API interface
}

impl NetworkConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: NetworkConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }
}

 // The  NetworkConfig  struct now has two new fields:  max_connections  and  api_enabled . The  max_connections  field is used to specify the maximum number of connections that the node can have. The  api_enabled  field is used to toggle the API interface on or off.
 // The  load_from_file  function now reads the  max_connections  and  api_enabled  fields from the JSON file.
 // Next, we need to update the  main  function in  src/main.rs  to use the new fields.