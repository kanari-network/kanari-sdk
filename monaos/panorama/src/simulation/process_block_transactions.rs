// Helper function to process transactions in a block and return valid ones with total fees

use consensus_pos::Blake3Algorithm;

use crate::{block::Block, blockchain::BALANCES, transaction::Transaction};

use super::{get_mutex_lock, SimulationError};
use log::{error, debug};

pub fn process_block_transactions(block: &Block<Blake3Algorithm>) -> Result<(Vec<Transaction>, u64), SimulationError> {
    let mut valid_transactions = Vec::new();
    let mut total_fees = 0;
    
    // Pre-filter system transactions to reduce lock time
    for tx in &block.transactions {
        if tx.sender == "system" {
            valid_transactions.push(tx.clone());
        }
    }
    
    // Process non-system transactions
    let mut balances = get_mutex_lock(&BALANCES, "process_transactions")?;
    
    for tx in &block.transactions {
        // Skip system transactions as they've already been processed
        if tx.sender == "system" {
            continue;
        }

        // Check if sender exists and has sufficient funds
        if let Some(sender_balance) = balances.get_mut(&tx.sender) {
            let total_cost = tx.amount + tx.gas_cost as u64;
            
            if *sender_balance >= total_cost {
                // Transaction is valid, process it
                *sender_balance -= total_cost;
                
                // Credit the receiver
                *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                
                // Add transaction fee to total
                total_fees += tx.gas_cost as u64;
                
                // Add to valid transactions list
                valid_transactions.push(tx.clone());
            } else {
                // Log insufficient funds
                error!("Transaction failed: insufficient funds. Required: {}, Available: {}, Tx: {}", 
                       total_cost, *sender_balance, tx.hash);
                debug!("Transaction details: {:?}", tx);
            }
        } else {
            // Sender not found in balances
            error!("Transaction failed: sender address not found: {}, Tx: {}", tx.sender, tx.hash);
        }
    }
    
    Ok((valid_transactions, total_fees))
}