//! Config/env system: default configuration structure and paths

use kanari_common::config::{create_default_config, create_default_envs};

#[test]
fn default_config_has_required_keys() {
    let config = create_default_config();
    for key in ["keystore_path", "envs", "active_env", "active_address"] {
        assert!(
            config.get(key).is_some(),
            "default config must have key `{key}`"
        );
    }
    assert_eq!(
        config.get("active_env").and_then(|v| v.as_str()),
        Some("local")
    );
}

#[test]
fn default_envs_include_local_and_dev() {
    let envs = create_default_envs();
    let seq = envs.as_sequence().expect("envs must be a sequence");
    let aliases: Vec<&str> = seq
        .iter()
        .filter_map(|e| e.get("alias").and_then(|a| a.as_str()))
        .collect();
    assert!(aliases.contains(&"local"), "must include env `local`");
    assert!(aliases.contains(&"dev"), "must include env `dev`");

    for env in seq {
        let rpc = env
            .get("rpc")
            .and_then(|r| r.as_str())
            .expect("every env must have rpc url");
        assert!(
            rpc.starts_with("http"),
            "rpc url must start with http: {rpc}"
        );
    }
}

#[test]
fn kanari_paths_are_absolute() {
    let dir = kanari_common::get_kanari_dir();
    let cfg = kanari_common::get_kanari_config_path();
    assert!(dir.is_absolute(), "kanari dir must be absolute path");
    assert!(cfg.is_absolute(), "config path must be absolute path");
}
