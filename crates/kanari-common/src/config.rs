// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{get_kanari_config_path, get_kanari_dir};
use serde_yaml::{Mapping, Value};
use std::fs::{self, File};
use std::io::{self, Write};

/// Load configuration from kanari.yaml file
pub fn load_kanari_config() -> io::Result<Value> {
    let config_path = get_kanari_config_path();

    // Return default config if file doesn't exist
    if !config_path.exists() {
        let default_config = create_default_config();
        save_kanari_config(&default_config)?;
        return Ok(default_config);
    }

    // Read and parse config file
    let config_str = fs::read_to_string(&config_path)?;

    // Return default config if file is empty
    if config_str.trim().is_empty() {
        let default_config = create_default_config();
        save_kanari_config(&default_config)?;
        return Ok(default_config);
    }

    // Parse YAML with error handling
    let mut config: Value = serde_yaml::from_str(&config_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse kanari.yaml file: {}", e),
        )
    })?;

    // Ensure it has the default structure if parts are missing
    let mut changed = false;
    if let Some(map) = config.as_mapping_mut() {
        // Check for keystore_path
        if !map.contains_key(Value::String("keystore_path".to_string())) {
            let mut keystore_path = get_kanari_dir();
            keystore_path.push("kanari_config");
            keystore_path.push("kanari.keystore");
            map.insert(
                Value::String("keystore_path".to_string()),
                Value::String(keystore_path.to_string_lossy().to_string()),
            );
            changed = true;
        }

        // Check for envs
        if !map.contains_key(Value::String("envs".to_string())) {
            let default_envs = create_default_envs();
            map.insert(Value::String("envs".to_string()), default_envs);
            changed = true;
        }

        // Check for active_env
        if !map.contains_key(Value::String("active_env".to_string())) {
            map.insert(
                Value::String("active_env".to_string()),
                Value::String("local".to_string()),
            );
            changed = true;
        }
    }

    if changed {
        save_kanari_config(&config)?;
    }

    Ok(config)
}

/// Save configuration to kanari.yaml file
pub fn save_kanari_config(config: &Value) -> io::Result<()> {
    let config_path = get_kanari_config_path();

    // Serialize and save with error handling
    let yaml_str = serde_yaml::to_string(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize config: {}", e),
        )
    })?;

    // Atomic write using a temporary file
    let tmp_path = config_path.with_extension("tmp");
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(yaml_str.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(tmp_path, config_path)?;

    Ok(())
}

/// Create default configuration matching user expectations
pub fn create_default_config() -> Value {
    let mut config = Mapping::new();

    // Set keystore path
    let mut keystore_path = get_kanari_dir();
    keystore_path.push("kanari_config");
    keystore_path.push("kanari.keystore");
    config.insert(
        Value::String("keystore_path".to_string()),
        Value::String(keystore_path.to_string_lossy().to_string()),
    );

    // Set default environments
    config.insert(Value::String("envs".to_string()), create_default_envs());

    // Set active environment
    config.insert(
        Value::String("active_env".to_string()),
        Value::String("local".to_string()),
    );

    // Set active address (default wallet)
    config.insert(Value::String("active_address".to_string()), Value::Null);

    Value::Mapping(config)
}

/// Helper to create default environments sequence
pub fn create_default_envs() -> Value {
    let mut envs = Vec::new();

    let env_list = [
        ("local", "http://127.0.0.1:6767"),
        // ("dev", "http://127.0.0.1:19001"),
        ("dev", "https://dev-seed.kanari.network"),
        // ("test", "https://test-seed.rooch.network"),
        // ("main", "https://main-seed.kanari.network"),
    ];

    for (alias, rpc) in env_list {
        let mut env = Mapping::new();
        env.insert(
            Value::String("alias".to_string()),
            Value::String(alias.to_string()),
        );
        env.insert(
            Value::String("rpc".to_string()),
            Value::String(rpc.to_string()),
        );
        env.insert(Value::String("ws".to_string()), Value::Null);
        envs.push(Value::Mapping(env));
    }

    Value::Sequence(envs)
}
