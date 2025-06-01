// src/config.rs
use std::io;
use network::NetworkConfig;

// Re-export all functions from common for backward compatibility
pub use common::{
    load_config, 
    save_config, 
    load_kanari_config, 
    save_kanari_config,
    configure_network_settings,
    get_network_config,
    update_network_config,
    prompt_for_value,
    setup_network_config,
    ensure_network_config,
    init_default_config
};

/// Function to configure the network settings (backward compatibility wrapper)
pub fn configure_network(chain_id: &str) -> io::Result<NetworkConfig> {
    setup_network_config(chain_id)
}