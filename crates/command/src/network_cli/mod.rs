use colored::Colorize;
use std::io::{self};
use serde_yaml::{Value, Mapping};
use common::{load_kanari_config, save_kanari_config};

pub fn handle_network_command() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        display_network_help();
        return Ok(());
    }
    
    match args[2].as_str() {
        "list" => list_networks()?,
        "switch" if args.len() >= 4 => switch_network(&args[3])?,
        "add" if args.len() >= 4 => add_network(&args[3], args.get(4).map(|s| s.as_str()).unwrap_or("http://127.0.0.1:30030"))?,
        "remove" if args.len() >= 4 => remove_network(&args[3])?,
        _ => display_network_help(),
    }
    
    Ok(())
}

fn display_network_help() {
    println!("\n{}", "KARI NETWORK MANAGEMENT".bright_yellow().bold());
    println!("{}", "Available commands:".bright_white());
    println!("  {:<20} {}", "list".green(), "List all available networks");
    println!("  {:<20} {}", "switch <name>".green(), "Switch to the specified network");
    println!("  {:<20} {}", "add <name> [url]".green(), "Add a new network with optional URL");
    println!("  {:<20} {}", "remove <name>".green(), "Remove a network configuration");
    println!("\n{}", "Examples:".bright_yellow());
    println!("  kari network list");
    println!("  kari network switch dev");
    println!("  kari network add local_test http://127.0.0.1:30035");
    println!("  kari network remove local_test");
}

fn list_networks() -> io::Result<()> {
    let kanari_config = load_kanari_config()?;
    
    let active_env = kanari_config.get("active_env")
                     .and_then(|v| v.as_str())
                     .unwrap_or("local");
                     
    println!("\n{}", "AVAILABLE NETWORKS".bright_yellow().bold());
    println!("{:<15} {:<40} {}", "NAME".bright_white(), "RPC URL".bright_white(), "STATUS".bright_white());
    
    if let Some(envs) = kanari_config.get("envs").and_then(|v| v.as_sequence()) {
        for env in envs {
            let alias = env.get("alias").and_then(|v| v.as_str()).unwrap_or("unknown");
            let rpc = env.get("rpc").and_then(|v| v.as_str()).unwrap_or("not configured");
            
            if alias == active_env {
                println!("{:<15} {:<40} {}", alias.green(), rpc, "ACTIVE".bright_green());
            } else {
                println!("{:<15} {:<40}", alias, rpc);
            }
        }
    } else {
        println!("No networks configured. Use 'kari network add' to add one.");
    }
    
    println!("");
    Ok(())
}

fn switch_network(name: &str) -> io::Result<()> {
    let mut kanari_config = load_kanari_config()?;
    
    // Verify the network exists
    let mut network_exists = false;
    if let Some(envs) = kanari_config.get("envs").and_then(|v| v.as_sequence()) {
        for env in envs {
            if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                if alias == name {
                    network_exists = true;
                    break;
                }
            }
        }
    }
    
    if !network_exists {
        println!("{}", format!("Network '{}' not found.", name).red());
        return Ok(());
    }
    
    // Update active environment
    if let Some(mapping) = kanari_config.as_mapping_mut() {
        mapping.insert(
            Value::String("active_env".to_string()),
            Value::String(name.to_string())
        );
    } else {
        println!("Failed to update configuration.");
        return Ok(());
    }
    
    // Save updated config
    save_kanari_config(&kanari_config)?;
    
    println!("{}", format!("Switched to network: {}", name).green());
    println!("Use 'kari start' to connect to this network.");
    
    Ok(())
}

fn add_network(name: &str, rpc_url: &str) -> io::Result<()> {
    let mut kanari_config = load_kanari_config()?;
    
    // Check if network already exists
    let mut network_exists = false;
    if let Some(envs) = kanari_config.get("envs").and_then(|v| v.as_sequence()) {
        for env in envs {
            if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                if alias == name {
                    network_exists = true;
                    break;
                }
            }
        }
    }
    
    if network_exists {
        println!("{}", format!("Network '{}' already exists. Use a different name.", name).red());
        return Ok(());
    }
    
    // Create new network entry
    let mut new_env = Mapping::new();
    new_env.insert(Value::String("alias".to_string()), Value::String(name.to_string()));
    new_env.insert(Value::String("rpc".to_string()), Value::String(rpc_url.to_string()));
    new_env.insert(Value::String("ws".to_string()), Value::Null);
    
    // Add to environments list
    if let Some(mapping) = kanari_config.as_mapping_mut() {
        if let Some(envs) = mapping.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
            envs.push(Value::Mapping(new_env));
        } else {
            // No environments yet, create the list
            mapping.insert(
                Value::String("envs".to_string()),
                Value::Sequence(vec![Value::Mapping(new_env)])
            );
        }
    } else {
        // Empty config, create everything
        let mut new_config = Mapping::new();
        new_config.insert(
            Value::String("envs".to_string()),
            Value::Sequence(vec![Value::Mapping(new_env)])
        );
        kanari_config = Value::Mapping(new_config);
    }
    
    // Save updated config
    save_kanari_config(&kanari_config)?;
    
    println!("{}", format!("Added network: {} ({})", name, rpc_url).green());
    println!("To use this network, run: kari network switch {}", name);
    
    Ok(())
}

fn remove_network(name: &str) -> io::Result<()> {
    // Don't allow removing primary networks
    if name == "local" || name == "dev" || name == "test" || name == "main" {
        println!("{}", format!("Cannot remove built-in network '{}'.", name).red());
        return Ok(());
    }
    
    let mut kanari_config = load_kanari_config()?;
    
    let active_env = kanari_config.get("active_env")
                     .and_then(|v| v.as_str())
                     .unwrap_or("local");
                     
    // Check if trying to remove active network
    if active_env == name {
        println!("{}", "Cannot remove currently active network.".red());
        println!("Switch to another network first with: kari network switch <name>");
        return Ok(());
    }
    
    // Remove the network
    let mut network_found = false;
    if let Some(mapping) = kanari_config.as_mapping_mut() {
        if let Some(envs) = mapping.get_mut("envs").and_then(|v| v.as_sequence_mut()) {
            let mut index_to_remove = None;
            for (i, env) in envs.iter().enumerate() {
                if let Some(alias) = env.get("alias").and_then(|v| v.as_str()) {
                    if alias == name {
                        index_to_remove = Some(i);
                        network_found = true;
                        break;
                    }
                }
            }
            
            if let Some(index) = index_to_remove {
                envs.remove(index);
            }
        }
    }
    
    if !network_found {
        println!("{}", format!("Network '{}' not found.", name).red());
        return Ok(());
    }
    
    // Save updated config
    save_kanari_config(&kanari_config)?;
    
    println!("{}", format!("Removed network: {}", name).green());
    
    Ok(())
}
