use tokio::sync::mpsc;
use log::{info, error, debug};
use rand::{Rng, thread_rng};
use std::sync::RwLock;
use lazy_static::lazy_static;

use crate::block::Transaction;
use crate::simulation::add_pending_transaction;

// Gas fee collector address remains constant
pub const GAS_FEE_COLLECTOR: &str = "0x47621776628ba3a5b9baaab38e61f4c98e893e124204bc4dad52e702e2b24ea1";

// Gas fee configuration
pub const MIN_GAS_FEE: u64 = 2;      // Minimum gas fee (0.000000002 KA)
pub const BASE_GAS_FEE: u64 = 3;     // Base gas fee (0.000000003 KA)
pub const MAX_GAS_FEE: u64 = 50;     // Maximum gas fee (0.000000050 KA)
pub const CONGESTION_MULTIPLIER: f64 = 0.001; // How much each pending tx affects fee

// Store network statistics for gas calculation
lazy_static! {
    pub static ref NETWORK_STATS: RwLock<NetworkStats> = RwLock::new(NetworkStats::default());
}

pub struct NetworkStats {
    pub pending_transactions: usize,
    pub last_block_time: u64,
    pub transaction_count_24h: usize,
}

impl Default for NetworkStats {
    fn default() -> Self {
        NetworkStats {
            pending_transactions: 0,
            last_block_time: 0,
            transaction_count_24h: 0,
        }
    }
}

// Update network stats with pending transaction count
pub fn update_pending_transaction_count(count: usize) {
    if let Ok(mut stats) = NETWORK_STATS.write() {
        stats.pending_transactions = count;
    }
}

// Update network stats with last block time
pub fn update_last_block_time(timestamp: u64) {
    if let Ok(mut stats) = NETWORK_STATS.write() {
        stats.last_block_time = timestamp;
    }
}

// Update 24h transaction count
pub fn update_transaction_count_24h(count: usize) {
    if let Ok(mut stats) = NETWORK_STATS.write() {
        stats.transaction_count_24h = count;
    }
}

/// Calculate gas fee dynamically based on network conditions
pub fn calculate_gas_fee(priority_boost: Option<u64>) -> u64 {
    let network_stats = match NETWORK_STATS.read() {
        Ok(stats) => stats,
        Err(_) => return BASE_GAS_FEE, // Default to base fee if can't read stats
    };
    
    // Calculate congestion component based on pending transactions
    let congestion_fee = ((network_stats.pending_transactions as f64) * CONGESTION_MULTIPLIER) as u64;
    
    // Apply user's priority boost if provided
    let priority = priority_boost.unwrap_or(0);
    
    // Calculate total gas fee
    let gas_fee = BASE_GAS_FEE + congestion_fee + priority;
    
    // Ensure gas fee is within allowed range
    let gas_fee = gas_fee.clamp(MIN_GAS_FEE, MAX_GAS_FEE);
    
    debug!("Calculated gas fee: {} (base: {}, congestion: {}, priority: {}, pending txs: {})",
           gas_fee, BASE_GAS_FEE, congestion_fee, priority, network_stats.pending_transactions);
    
    gas_fee
}

/// Generate a random transaction ID (0x followed by 64 random hex characters)
pub fn generate_transaction_id() -> String {
    let mut rng = thread_rng();
    let mut id = String::with_capacity(66); // 0x + 64 chars
    id.push_str("0x");
    
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    for _ in 0..64 {
        let idx = rng.gen_range(0..HEX_CHARS.len());
        id.push(HEX_CHARS[idx] as char);
    }
    
    id
}

/// Process a blockchain transfer with improved reliability
pub fn process_blockchain_transfer(
    from_address: &str,
    to_address: &str,
    amount: u64,
    password: &str,
    priority_boost: Option<u64>,
    notification_channel: Option<&mpsc::Sender<String>>
) -> Result<Transaction, String> {
    // Calculate the gas fee dynamically
    let gas_fee = calculate_gas_fee(priority_boost);
    
    // Perform the transfer
    match crate::transfer_tokens::transfer_tokens(from_address, to_address, amount, password, gas_fee) {
        Ok(transaction) => {
            // Add to pending transactions queue
            if add_pending_transaction(transaction.clone()) {
                info!("Transaction added to pending queue: {} -> {}, amount: {}, gas fee: {}", 
                      transaction.sender, transaction.receiver, amount, gas_fee);
                
                // Send notification if channel provided
                if let Some(tx) = notification_channel {
                    let tx_json = serde_json::json!({
                        "event": "transaction_created",
                        "transaction": {
                            "id": transaction.transaction_id,
                            "sender": transaction.sender.to_hex_literal(),
                            "receiver": transaction.receiver.to_hex_literal(),
                            "amount": amount,
                            "gas_fee": gas_fee,
                            "timestamp": transaction.timestamp,
                            "signed": !transaction.signature.is_empty()
                        },
                        "status": "pending"
                    }).to_string();
                    
                    let _ = tx.try_send(tx_json);
                }
                
                Ok(transaction)
            } else {
                error!("Failed to add transaction to pending queue");
                Err("Failed to add transaction to pending queue".to_string())
            }
        },
        Err(e) => {
            error!("Transaction failed: {}", e);
            
            // Send notification if channel provided
            if let Some(tx) = notification_channel {
                let error_json = serde_json::json!({
                    "event": "transaction_error",
                    "error": format!("{}", e),
                    "details": {
                        "sender": from_address,
                        "receiver": to_address,
                        "amount": amount,
                        "gas_fee": gas_fee,
                    }
                }).to_string();
                
                let _ = tx.try_send(error_json);
            }
            
            Err(format!("Transaction failed: {}", e))
        }
    }
}

/// Format KA amount to display as KARI with proper formatting
pub fn format_kari_amount(ka_amount: u64) -> String {
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

/// Calculate total amount needed for a transaction including gas
pub fn calculate_total_transaction_cost(amount: u64, gas_fee: u64) -> u64 {
    amount + gas_fee
}

/// Validate that the user has enough funds for amount + gas fee
pub fn validate_transaction_funds(balance: u64, amount: u64, gas_fee: u64) -> bool {
    balance >= calculate_total_transaction_cost(amount, gas_fee)
}

/// Get current network statistics for gas fee calculation
pub fn get_network_stats() -> NetworkStats {
    match NETWORK_STATS.read() {
        Ok(stats) => stats.clone(),
        Err(_) => NetworkStats::default(),
    }
}

// Make NetworkStats cloneable
impl Clone for NetworkStats {
    fn clone(&self) -> Self {
        Self {
            pending_transactions: self.pending_transactions,
            last_block_time: self.last_block_time,
            transaction_count_24h: self.transaction_count_24h,
        }
    }
}
