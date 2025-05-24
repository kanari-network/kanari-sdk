use common::{get_kanari_config_path, save_kanari_config, get_kari_dir};
use serde_yaml::{Value, Mapping};
use std::io;
use colored::Colorize;

fn default_keystore_path() -> String {
    let mut path = get_kari_dir();
    path.push("kanari_config");
    path.push("kanari.keystore");
    path.to_string_lossy().into_owned()
}

fn create_env_config(alias: &str, rpc: &str, ws: &str) -> Value {
    let mut env_map = Mapping::new();
    env_map.insert(Value::String("alias".to_string()), Value::String(alias.to_string()));
    env_map.insert(Value::String("rpc".to_string()), Value::String(rpc.to_string()));
    env_map.insert(Value::String("ws".to_string()), Value::String(ws.to_string()));
    Value::Mapping(env_map)
}

pub fn init_server_config() -> io::Result<()> {
    let config_path = get_kanari_config_path();

    if config_path.exists() {
        println!("{}", "kanari.yaml already exists. Initialization skipped.".yellow());
        return Ok(());
    }

    let mut config = Mapping::new();
    config.insert(
        Value::String("keystore_path".to_string()),
        Value::String(default_keystore_path()),
    );
    config.insert(
        Value::String("active_address".to_string()),
        Value::Null,
    );

    let envs = vec![
        create_env_config("local", "http://127.0.0.1:30030", "ws://127.0.0.1:30031"),
        create_env_config("dev", "https://dev-seed.kanari.site", "wss://dev-seed.kanari.site/websocket"),
        create_env_config("test", "https://test-seed.kanari.site", "wss://test-seed.kanari.site/websocket"),
        create_env_config("main", "https://main-seed.kanari.site", "wss://main-seed.kanari.site/websocket"),
    ];

    config.insert(Value::String("envs".to_string()), Value::Sequence(envs));
    config.insert(Value::String("active_env".to_string()), Value::String("local".to_string()));

    let yaml_value = Value::Mapping(config);
    save_kanari_config(&yaml_value)?;

    println!("{} {}", "Default kanari.yaml created at:".green(), config_path.display().to_string().bright_white());
    Ok(())
}
