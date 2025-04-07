use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs}; // Add ToSocketAddrs trait
use futures::FutureExt;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{ServerBuilder, AccessControlAllowOrigin, DomainsValidation};
use metadata::{get_file, upload_file};
use network::NetworkConfig;
use colored::Colorize; // Add Colorize trait for color methods
mod metadata; // Add this at the top with the other modules
mod stake;
mod get_block; // Add the new module
mod accounts;

use stake::{get_staking_info, get_staking_stats, stake_tokens, unstake_tokens};

// Use functions from the get_block module
use get_block::{
    get_all_blocks, 
    get_account_details, 
    search_transactions, 
    get_transaction_by_id, 
    get_transaction_status, 
    get_gas_fee_info
};

use accounts::{get_blockchain_status, get_wallets, list_accounts, transfer_tokens};

// Format function locally since panorama::utils is not available
fn format_kari_amount(ka_amount: u64) -> String {
    const KA_PER_KARI: u64 = 1_000_000_000;
    
    // Calculate whole and fractional parts
    let whole_kari = ka_amount / KA_PER_KARI;
    let fractional_ka = ka_amount % KA_PER_KARI;
    
    // Format with thousands separators and 9 decimal places
    let whole_formatted = format!("{}", whole_kari)
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>();
    
    format!("{}.{:09}", whole_formatted, fractional_ka)
}

/// Starts the RPC server for file operations
pub async fn start_rpc_server(network_config: NetworkConfig) -> Result<(), tokio::io::Error> {
    let mut io = IoHandler::new();

    // Add file operations
    io.add_method("upload_file", |params| {
        futures::future::ready(upload_file(params)).boxed()
    });

    io.add_method("get_file", |params| {
        futures::future::ready(get_file(params)).boxed()
    });

    // Add blockchain operations
    io.add_method("blockchain_status", |params| {
        futures::future::ready(get_blockchain_status(params)).boxed()
    });
    
    
    io.add_method("list_accounts", |params| {
        futures::future::ready(list_accounts(params)).boxed()
    });
    
    io.add_method("transfer", |params| {
        futures::future::ready(transfer_tokens(params)).boxed()
    });
    
    io.add_method("get_wallets", |params| {
        futures::future::ready(get_wallets(params)).boxed()
    });

    // Use methods from get_block.rs
    io.add_method("get_all_blocks", |params| {
        futures::future::ready(get_all_blocks(params)).boxed()
    });
    
    io.add_method("get_account_details", |params| {
        futures::future::ready(get_account_details(params)).boxed()
    });
    
    io.add_method("search_transactions", |params| {
        futures::future::ready(search_transactions(params)).boxed()
    });

    io.add_method("get_transaction_by_id", |params| {
        futures::future::ready(get_transaction_by_id(params)).boxed()
    });

    io.add_method("get_transaction_status", |params| {
        futures::future::ready(get_transaction_status(params)).boxed()
    });

    io.add_method("get_gas_fee_info", |params| {
        futures::future::ready(get_gas_fee_info(params)).boxed()
    });

    // Add staking operations
    io.add_method("stake_tokens", |params| {
        futures::future::ready(stake_tokens(params)).boxed()
    });

    io.add_method("unstake_tokens", |params| {
        futures::future::ready(unstake_tokens(params)).boxed()
    });

    io.add_method("get_staking_info", |params| {
        futures::future::ready(get_staking_info(params)).boxed()
    });

    io.add_method("get_staking_stats", |params| {
        futures::future::ready(get_staking_stats(params)).boxed()
    });

    // Configure socket address to bind to all interfaces
    let bind_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), // Bind to all interfaces
        network_config.port
    );

    // Create CORS settings for production
    let allowed_origins = vec![
        AccessControlAllowOrigin::Any, // Allow all origins for testing
    ];

    // Check if TLS certificates are available - FIXED: Clearer messaging about TLS
    let kari_dir = common::get_kari_dir();
    let cert_path = kari_dir.join("certs").join("node.crt");
    let key_path = kari_dir.join("certs").join("node.key");
    
    let use_tls = cert_path.exists() && key_path.exists();
    
    if use_tls {
        println!("TLS certificates found at:");
        println!("  - Certificate: {}", cert_path.display());
        println!("  - Key: {}", key_path.display());
        println!("Note: For TLS support, please use a reverse proxy like Nginx or Caddy.");
        println!("      The built-in server only supports HTTP.");
    } else {
        println!("TLS certificates not found. Running in HTTP mode.");
        println!("To set up TLS, run 'kari certificate generate' and configure a reverse proxy.");
    }
    
    // Start HTTP server
    match ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(allowed_origins))
        .threads(4) // Increase thread count for better performance
        .max_request_body_size(10 * 1024 * 1024) // 10MB max request size
        .health_api(("health", "ready")) // Add health check endpoints
        .start_http(&bind_addr)
    {
        Ok(server) => {
            // Check for domain configuration and load it from config
            let domain = common::get_node_domain();
            
            // Display server connection information with domain if available
            if let Some(domain_name) = domain {
                println!("HTTP server running on http://{}:{}", network_config.node_address, network_config.port);
                
                // If domain includes port, extract just the hostname
                let domain_parts: Vec<&str> = domain_name.split(':').collect();
                let hostname = domain_parts[0];
                let domain_port = if domain_parts.len() > 1 {
                    domain_parts[1]
                } else {
                    "80"
                };
                
                println!("Domain configured: {}", domain_name.bright_green());
                println!("You can access the RPC API at:");
                println!("  - http://{}:{} (direct IP)", network_config.node_address, network_config.port);
                
                // Fix: Use conditional printing to avoid temporary value issue
                if domain_port == "80" {
                    println!("  - http://{} (domain)", hostname);
                } else {
                    println!("  - http://{}:{} (domain)", hostname, domain_port);
                }
                
                // For kanari.site domains, suggest how other nodes can connect
                if hostname.contains("kanari.site") {
                    println!("\nTo connect other nodes to this node:");
                    println!("  kari start --peer {}:51303", hostname);
                }
                
                // Try to resolve the domain name to verify it's pointed to this server
                if let Ok(addrs) = format!("{}:80", hostname).to_socket_addrs() {
                    let resolved_ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                    if !resolved_ips.is_empty() {
                        println!("Domain resolves to: {}", resolved_ips.join(", "));
                        
                        // Get the local IP to compare
                        if let Some(local_ip) = panorama::node::get_local_ip() {
                            if resolved_ips.contains(&local_ip) {
                                println!("✓ Domain correctly resolves to this server's IP ({})", local_ip);
                            } else {
                                println!("⚠ Domain doesn't resolve to this server's IP ({})", local_ip);
                                println!("  To fix: Update DNS A record to point to your server's IP address");
                            }
                        }
                    } else {
                        println!("⚠ Could not resolve domain. DNS might not be properly configured.");
                    }
                }
            } else {
                println!("HTTP server running on http://{}:{}", network_config.node_address, network_config.port);
                println!("No domain configured. To set up a domain:");
                println!("  1. Register a domain and set up DNS to point to your server's IP");
                println!("  2. Add 'domain: \"your-domain.com\"' to your ~/.kari/config.yaml file");
            }
            
            if !network_config.peers.is_empty() {
                println!("Connected to peers:");
                for peer in &network_config.peers {
                    println!("  - {}", peer);
                }
            }
            
            // Create a channel for shutdown coordination
            let (shutdown_complete_tx, _shutdown_complete_rx) = tokio::sync::oneshot::channel();
            
            // Spawn a task to wait for the server in the background
            tokio::spawn(async move {
                server.wait();
                let _ = shutdown_complete_tx.send(());
            });
            
            // Return immediately, allowing the server to run in the background
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to start HTTP server: {}", e);
            Err(e.into())
        }
    }
}