// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{load_kanari_config, save_kanari_config};
use serde_yaml::Value;
use std::io;

/// Get active wallet address
pub fn get_active_address() -> Option<String> {
    load_kanari_config().ok().and_then(|config| {
        config
            .get("active_address")
            .and_then(|v| v.as_str())
            .map(String::from)
    })
}

/// Set active wallet address
pub fn set_active_address(address: &str) -> io::Result<()> {
    let mut config = load_kanari_config()?;
    if let Some(map) = config.as_mapping_mut() {
        map.insert(
            Value::String("active_address".to_string()),
            Value::String(address.to_string()),
        );
        save_kanari_config(&config)?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid config format",
        ))
    }
}

/// Get current main wallet address (deprecated: use get_active_address instead)
pub fn get_main_wallet() -> Option<String> {
    get_active_address()
}
