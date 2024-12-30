// src/config.rs
use std::io::{self, Write};
use std::path::PathBuf;
use serde_json::{json, Value};
use std::fs::{self, File};
use dirs;
use network::{NetworkConfig, NetworkType};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;


// Function to get the configuration directory
pub fn get_config_dir() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir().expect("Unable to find home directory");
    let config_path = home_dir.join(".kari").join("network");
    fs::create_dir_all(&config_path)?;
    Ok(config_path)
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
    
    // Parse the JSON, convert parsing errors to io::Error
    serde_json::from_str(&config_str).map_err(|e| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Failed to parse config file: {}", e)
    ))
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
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        default.to_string()
    } else {
        trimmed_input.to_string()
    }
}

// Function to configure the network settings
pub fn configure_network(chain_id: &str) -> io::Result<NetworkConfig> {
    let mut config = load_config()?;

    // Check if the configuration already exists
    if config.get("network_type").is_some() && config.get("rpc_port").is_some() && 
       config.get("domain").is_some() && config.get("chain_id").is_some() {
        println!("Configuration already exists. Skipping configuration process.");
        let network_type = match config.get("network_type").unwrap().as_str().unwrap() {
            "devnet" => NetworkType::Devnet,
            "testnet" => NetworkType::Testnet,
            "mainnet" => NetworkType::Mainnet,
            _ => NetworkType::Mainnet,
        };

        let _domain = config.get("domain").unwrap().as_str().unwrap().to_string();
        let chain_id = config.get("chain_id").unwrap().as_str().unwrap().to_string();

        return Ok(NetworkConfig {
            node_address: "127.0.0.1".to_string(),
            domain: config.get("domain").unwrap().as_str().unwrap().to_string(),
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

    // Function to check if a domain is reachable
    fn is_domain_reachable(domain: &str) -> bool {
        TcpStream::connect_timeout(&(domain, 80).to_socket_addrs().unwrap().next().unwrap(), Duration::from_secs(5)).is_ok()
    }

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

    config.as_object_mut().unwrap().insert("network_type".to_string(), json!(network_type_input));

    let default_rpc_port = match network_type_input {
        "devnet" => "3031",
        "testnet" => "3032",
        "mainnet" => "3030",
        _ => "3030", // Default to mainnet port
    };

    let rpc_port = prompt_for_value("Enter RPC port", default_rpc_port)
        .parse::<u16>()
        .expect("Invalid port number");
    config.as_object_mut().unwrap().insert("rpc_port".to_string(), json!(rpc_port));

    let default_domain = match network_type_input {
        "devnet" => "devnet.kanari.network",
        "testnet" => "testnet.kanari.network",
        "mainnet" => "mainnet.kanari.network",
        _ => "mainnet.kanari.network", // Default to mainnet domain
    };

    // Check if the default domain is reachable
    if !is_domain_reachable(default_domain) {
        eprintln!("The domain {} is not reachable. Please check your network connection or try again later.", default_domain);
        return Err(io::Error::new(io::ErrorKind::Other, "Domain not reachable"));
    }
    
    // Prompt the user for the network domain
    let domain = prompt_for_value("Enter network domain", default_domain);
    config.as_object_mut().unwrap().insert("domain".to_string(), json!(domain));

    // Save the chain ID to the configuration
    config.as_object_mut().unwrap().insert("chain_id".to_string(), json!(chain_id));

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

    save_config(&config)?;

    println!("Network configuration saved successfully.");
    Ok(network_config) // Return the NetworkConfig
}