use tokio::sync::mpsc;
use log::{info, error};
use rand::{Rng, thread_rng};

use crate::blockchain::transfer_tokens;
use crate::block::Transaction;
use crate::simulation::add_pending_transaction;

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
    notification_channel: Option<&mpsc::Sender<String>>
) -> Result<Transaction, String> {
    // Perform the transfer
    match transfer_tokens(from_address, to_address, amount) {
        Ok(transaction) => {
            // Add to pending transactions queue
            if add_pending_transaction(transaction.clone()) {
                info!("Transaction added to pending queue: {} -> {}, amount: {}", 
                      transaction.sender, transaction.receiver, amount);
                
                // Send notification if channel provided
                if let Some(tx) = notification_channel {
                    let tx_json = serde_json::json!({
                        "event": "transaction_created",
                        "transaction": {
                            "sender": transaction.sender.to_hex_literal(),
                            "receiver": transaction.receiver.to_hex_literal(),
                            "amount": amount,
                            "timestamp": transaction.timestamp
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
                        "amount": amount
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
