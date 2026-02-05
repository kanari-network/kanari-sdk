// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Configuration management for Kanari SDK
pub mod address;
pub mod config;
pub mod envs;

// Re-export core functions for backward compatibility
pub use address::{get_active_address, get_main_wallet, set_active_address};
pub use config::{
    create_default_config, create_default_envs, load_kanari_config, save_kanari_config,
};
pub use envs::{add_env, get_active_env, get_active_rpc, get_envs, remove_env, set_active_env};

use std::fs;
use std::path::PathBuf;

// Path utility functions
pub fn get_kanari_dir() -> PathBuf {
    dirs::home_dir()
        .map(|mut path| {
            path.push(".kanari");
            let _ = fs::create_dir_all(&path);
            path
        })
        .unwrap_or_else(|| PathBuf::from(".kanari"))
}

/// Get path to the kanari.yaml configuration file
pub fn get_kanari_config_path() -> PathBuf {
    let mut config_dir = get_kanari_dir();
    config_dir.push("kanari_config");
    let _ = fs::create_dir_all(&config_dir);
    config_dir.push("kanari.yaml");
    config_dir
}
