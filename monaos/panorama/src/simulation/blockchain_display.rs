use log::warn;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use mona_blockchain::block::Block;
use mona_types::kari::{KA_PER_KARI, POOL_ADDRESS, VALIDATOR_STAKING_MINIMUM_KARI, NODE_STAKING_MINIMUM_KARI};
use consensus_pos::Blake3Algorithm;

// Send blockchain initialization message
pub fn send_initialization_status(
    tx: &mpsc::Sender<String>, 
    coin_name: &str,
    coin_symbol: &str, 
    coin_decimals: u8,
    coin_total_supply: u64,
    total_supply_kari: f64,
    node_address: &str
) {
    let init_status = json!({
        "event": "blockchain_initializing",
        "coin": {
            "name": coin_name,
            "symbol": coin_symbol,
            "decimals": coin_decimals,
            "total_supply": coin_total_supply,
            "display_supply": total_supply_kari
        },
        "node_address": node_address,
        "staking": {
            "validator_minimum": VALIDATOR_STAKING_MINIMUM_KARI,
            "node_minimum": NODE_STAKING_MINIMUM_KARI,
        },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }).to_string();
    
    if let Err(e) = tx.try_send(init_status) {
        warn!("Failed to send blockchain initialization status: {}", e);
    }
}

// Send blockchain loaded status
pub fn send_blockchain_loaded_status(
    tx: &mpsc::Sender<String>,
    block_count: usize,
    last_block: &Block<Blake3Algorithm>
) {
    let status_json = json!({
        "event": "blockchain_loaded",
        "blocks": block_count,
        "last_block": {
            "index": last_block.index,
            "hash": last_block.hash,
            "timestamp": last_block.timestamp
        },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }).to_string();
    
    if let Err(e) = tx.try_send(status_json) {
        warn!("Failed to send blockchain loaded status: {}", e);
    }
}

// Send genesis block created notification
pub fn send_genesis_created_notification(
    tx: &mpsc::Sender<String>,
    genesis_block: &Block<Blake3Algorithm>,
    coin_name: &str,
    coin_symbol: &str,
    coin_decimals: u8,
    minter_address: &str,
    total_supply_ka: u64,
    total_supply_kari: f64
) {
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(genesis_block.timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown time".to_string());
        
    let genesis_json = json!({
        "event": "genesis_created",
        "block": {
            "index": genesis_block.index,
            "hash": genesis_block.hash,
            "timestamp": genesis_block.timestamp,
            "datetime": datetime
        },
        "coin": {
            "name": coin_name,
            "symbol": coin_symbol,
            "decimals": coin_decimals
        },
        "minter": minter_address,
        "total_supply": {
            "amount": total_supply_ka,
            "display": total_supply_kari,
            "symbol": coin_symbol
        }
    }).to_string();
    
    if let Err(e) = tx.try_send(genesis_json) {
        warn!("Failed to send genesis creation notification: {}", e);
    }
}

// Send block mined notification
pub fn send_block_mined_notification(
    tx: &mpsc::Sender<String>,
    block: &Block<Blake3Algorithm>,
    blockchain_height: usize,
    minter_address: &str,
    staking_rewards: u64,
    is_validator: bool,
    pool_balance: Option<u64>,
    peer_count: usize
) {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(block.timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown time".to_string());

    let status_json = json!({
        "event": "block_mined",
        "block": {
            "index": block.index,
            "hash": block.hash,
            "prev_hash": block.prev_hash,
            "timestamp": block.timestamp,
            "datetime": datetime,
            "minter": minter_address,
            "transactions": block.transactions.len(),
        },
        "blockchain": {
            "height": blockchain_height,
            "last_update": current_time
        },
        "staking": {
            "rewards_distributed": staking_rewards,
            "is_validator": is_validator,
            "display_rewards": staking_rewards as f64 / KA_PER_KARI as f64,
            "pool_balance": pool_balance.unwrap_or(0),
            "pool_balance_display": pool_balance.unwrap_or(0) as f64 / KA_PER_KARI as f64,
            "pool_address": POOL_ADDRESS
        },
        "networking": {
            "peer_count": peer_count,
            "node_id": format!("node-{}", minter_address[..8].to_string()),
        }
    }).to_string();
    
    if let Err(e) = tx.try_send(status_json) {
        warn!("Failed to send block mined notification: {}", e);
    }
}

// Send balance update
pub fn send_balance_update(
    tx: &mpsc::Sender<String>,
    address: &str,
    balance: u64,
    coin_symbol: &str,
    block_height: u64
) {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    let balance_display = balance as f64 / KA_PER_KARI as f64;
    
    let balance_json = json!({
        "event": "balance_update",
        "address": address,
        "balance": {
            "amount": balance,
            "display": balance_display,
            "symbol": coin_symbol,
            "formatted": format!("{:.9} {}", balance_display, coin_symbol)
        },
        "block_height": block_height,
        "timestamp": current_time
    }).to_string();
    
    if let Err(e) = tx.try_send(balance_json) {
        warn!("Failed to send balance update: {}", e);
    }
}

// Send system balances report
pub fn send_system_balances_report(tx: &mpsc::Sender<String>, balances_count: usize) {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    let balance_report = json!({
        "event": "system_balances",
        "account_count": balances_count,
        "timestamp": current_time
    }).to_string();
    
    if let Err(e) = tx.try_send(balance_report) {
        warn!("Failed to send system balances report: {}", e);
    }
}

// Send error notification
pub fn send_error_notification(
    tx: &mpsc::Sender<String>,
    error_type: &str,
    error_message: &str,
    details: Option<serde_json::Value>
) {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        
    let mut error_json = json!({
        "event": format!("{}_error", error_type),
        "error": error_message,
        "timestamp": current_time
    });
    
    // Add optional details if provided
    if let Some(detail_obj) = details {
        if let Some(obj) = error_json.as_object_mut() {
            obj.insert("details".to_string(), detail_obj);
        }
    }
    
    if let Err(e) = tx.try_send(error_json.to_string()) {
        warn!("Failed to send error notification: {}", e);
    }
}

// Send blockchain stopped notification
pub fn send_blockchain_stopped_notification(tx: &mpsc::Sender<String>) {
    let shutdown_json = json!({
        "event": "blockchain_stopped",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
    }).to_string();
    
    if let Err(e) = tx.try_send(shutdown_json) {
        warn!("Failed to send blockchain stopped notification: {}", e);
    }
}
