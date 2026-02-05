// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{load_kanari_config, save_kanari_config};
use serde_yaml::{Mapping, Value};
use std::io;

/// Get all environments from config
pub fn get_envs() -> io::Result<Vec<(String, String)>> {
    let config = load_kanari_config()?;
    let mut result = Vec::new();

    if let Some(envs) = config.get("envs").and_then(|v| v.as_sequence()) {
        result = envs
            .iter()
            .filter_map(|env| {
                let alias = env.get("alias").and_then(|v| v.as_str());
                let rpc = env.get("rpc").and_then(|v| v.as_str());
                if let (Some(alias), Some(rpc)) = (alias, rpc) {
                    Some((alias.to_string(), rpc.to_string()))
                } else {
                    None
                }
            })
            .collect();
    }
    Ok(result)
}

/// Get active environment alias
pub fn get_active_env() -> Option<String> {
    load_kanari_config().ok().and_then(|config| {
        config
            .get("active_env")
            .and_then(|v| v.as_str())
            .map(String::from)
    })
}

/// Set active environment by alias
pub fn set_active_env(alias: &str) -> io::Result<()> {
    let mut config = load_kanari_config()?;
    if let Some(map) = config.as_mapping_mut() {
        // Verify alias exists in envs
        let exists = map
            .get(Value::String("envs".to_string()))
            .and_then(|v| v.as_sequence())
            .map(|envs| {
                envs.iter()
                    .any(|env| env.get("alias").and_then(|v| v.as_str()) == Some(alias))
            })
            .unwrap_or(false);

        if !exists {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Environment alias '{}' not found", alias),
            ));
        }

        map.insert(
            Value::String("active_env".to_string()),
            Value::String(alias.to_string()),
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

/// Add or update an environment
pub fn add_env(alias: &str, rpc: &str) -> io::Result<()> {
    let mut config = load_kanari_config()?;
    if let Some(map) = config.as_mapping_mut() {
        let envs_key = Value::String("envs".to_string());
        if !map.contains_key(&envs_key) {
            map.insert(envs_key.clone(), Value::Sequence(Vec::new()));
        }

        if let Some(envs) = map.get_mut(&envs_key).and_then(|v| v.as_sequence_mut()) {
            // Check if already exists, then update
            let mut found = false;
            for env in envs.iter_mut() {
                if env.get("alias").and_then(|v| v.as_str()) == Some(alias)
                    && let Some(env_map) = env.as_mapping_mut()
                {
                    env_map.insert(
                        Value::String("rpc".to_string()),
                        Value::String(rpc.to_string()),
                    );
                    env_map.insert(Value::String("ws".to_string()), Value::Null);
                    found = true;
                    break;
                }
            }

            if !found {
                let mut new_env = Mapping::new();
                new_env.insert(
                    Value::String("alias".to_string()),
                    Value::String(alias.to_string()),
                );
                new_env.insert(
                    Value::String("rpc".to_string()),
                    Value::String(rpc.to_string()),
                );
                new_env.insert(Value::String("ws".to_string()), Value::Null);
                envs.push(Value::Mapping(new_env));
            }
        }

        save_kanari_config(&config)?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid config format",
        ))
    }
}

/// Remove an environment by alias
pub fn remove_env(alias: &str) -> io::Result<()> {
    let mut config = load_kanari_config()?;

    if let Some(map) = config.as_mapping_mut() {
        let mut removed = false;
        if let Some(envs) = map
            .get_mut(Value::String("envs".to_string()))
            .and_then(|v| v.as_sequence_mut())
        {
            let initial_len = envs.len();
            envs.retain(|env| env.get("alias").and_then(|v| v.as_str()) != Some(alias));
            removed = envs.len() < initial_len;
        }

        if removed {
            // If active_env was the one removed, clear it
            if map.get("active_env").and_then(|v| v.as_str()) == Some(alias) {
                map.remove(Value::String("active_env".to_string()));
            }
            save_kanari_config(&config)?;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Environment alias '{}' not found", alias),
            ))
        }
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid config format",
        ))
    }
}

/// Get RPC URL of the active environment
pub fn get_active_rpc() -> Option<String> {
    let config = load_kanari_config().ok()?;
    let active_alias = config.get("active_env").and_then(|v| v.as_str())?;

    if let Some(envs) = config.get("envs").and_then(|v| v.as_sequence()) {
        for env in envs {
            if env.get("alias").and_then(|v| v.as_str()) == Some(active_alias) {
                return env.get("rpc").and_then(|v| v.as_str()).map(String::from);
            }
        }
    }
    None
}
