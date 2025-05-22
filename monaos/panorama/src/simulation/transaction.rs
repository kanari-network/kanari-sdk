use log::{warn, info};
use std::sync::RwLock;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use serde_json::json;

use mona_blockchain::block::Transaction;
use mona_blockchain::blockchain::{normalize_address, save_blockchain};
use crate::transfer_tokens;
use crate::utils::{update_pending_transaction_count, calculate_gas_fee, format_gas_fee_display};

// Add pending transactions queue
lazy_static::lazy_static! {
    pub static ref PENDING_TRANSACTIONS: RwLock<VecDeque<Transaction>> = RwLock::new(VecDeque::new());
}

// Simplified pending transaction function
pub fn add_pending_transaction(transaction: Transaction) -> bool {
    PENDING_TRANSACTIONS.write()
        .map(|mut queue| {
            queue.push_back(transaction);
            update_pending_transaction_count(queue.len());
            true
        })
        .unwrap_or(false)
}

// Improved function to ensure transactions are committed properly
pub fn process_transfer(
    from_address: &str,
    to_address: &str,
    amount: u64,
    password: &str,
    priority_boost: Option<u64>,  // Add optional priority boost
    tx: &mpsc::Sender<String>
) -> Result<Transaction, String> {
    // Parse addresses
    let from = match normalize_address(from_address) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Invalid sender address: {}", e)),
    };
    
    let to = match normalize_address(to_address) {
        Ok(addr) => addr, 
        Err(e) => return Err(format!("Invalid receiver address: {}", e)),
    };
    
    // Calculate gas fee dynamically
    let gas_fee = calculate_gas_fee(priority_boost);
    let gas_fee_display = format_gas_fee_display(gas_fee);
    
    // Execute transfer using string representation and password for signing
    match transfer_tokens::transfer_tokens(&from.to_hex_literal(), &to.to_hex_literal(), amount, password, gas_fee) {
        Ok(transaction) => {
            // Verify signature right after creation for better debugging
            let signature_status = if transaction.signature.is_empty() {
                "unsigned"
            } else {
                match crate::transfer_tokens::verify_transaction::verify_transaction(&transaction) {
                    Ok(true) => "valid",
                    Ok(false) => "invalid",
                    Err(e) => {
                        warn!("Error verifying signature: {}", e);
                        "unknown"
                    }
                }
            };
            
            // Add to pending transactions
            if add_pending_transaction(transaction.clone()) {
                // Notify about successful transaction submission
                let tx_json = json!({
                    "event": "transaction_created",
                    "transaction": {
                        "id": transaction.transaction_id,
                        "sender": transaction.sender.to_hex_literal(),
                        "receiver": transaction.receiver.to_hex_literal(),
                        "amount": amount,
                        "gas_fee": transaction.gas_fee,
                        "gas_fee_display": format_gas_fee_display(transaction.gas_fee),
                        "gas_collector": crate::utils::GAS_FEE_COLLECTOR,
                        "total_cost": crate::utils::calculate_total_transaction_cost(amount, transaction.gas_fee),
                        "timestamp": transaction.timestamp,
                        "signed": !transaction.signature.is_empty(),
                        "signature_status": signature_status
                    },
                    "status": "pending"
                }).to_string();
                
                let _ = tx.try_send(tx_json);
                
                // Force save blockchain state to ensure transaction persistence
                match save_blockchain() {
                    Ok(_) => info!("Transaction recorded and blockchain state saved"),
                    Err(e) => warn!("Transaction recorded but failed to save state: {}", e),
                }
                
                // Return transaction
                Ok(transaction)
            } else {
                // Try to add the transaction directly to the next block
                match crate::simulation::force_transaction_inclusion(&transaction) {
                    true => {
                        info!("Transaction bypassed queue and directly included in blockchain");
                        Ok(transaction)
                    },
                    false => Err("Failed to add transaction to blockchain".to_string())
                }
            }
        },
        Err(e) => {
            // Update error JSON to use calculated gas fee
            let error_json = json!({
                "event": "transaction_error",
                "error": format!("{}", e),
                "details": {
                    "sender": from_address,
                    "receiver": to_address,
                    "amount": amount,
                    "gas_fee": gas_fee,
                    "gas_fee_display": gas_fee_display,
                    "total_cost": crate::utils::calculate_total_transaction_cost(amount, gas_fee)
                }
            }).to_string();
            
            let _ = tx.try_send(error_json);
            
            // Return error
            Err(format!("{}", e))
        }
    }
}
