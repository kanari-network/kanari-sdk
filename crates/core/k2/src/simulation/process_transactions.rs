use chrono::Local;
use colored::*;
use crossbeam::channel::RecvTimeoutError;
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mona_vm::TransactionStatus;

// Fix import paths for functions
use crate::simulation::process_transaction_in_vm::process_transaction_in_vm;
use crate::simulation::apply_state_changes::apply_state_changes;
use crate::simulation::apply_gas_charge::apply_gas_charge;
use crate::simulation::{get_mutex_lock, PENDING_TRANSACTIONS};

use super::{get_transaction_receiver, init_transaction_channel};



pub fn process_transactions(running: Arc<Mutex<bool>>) {
    // Initialize transaction channels if not already done
    init_transaction_channel();
    
    while *running.lock().unwrap() {
        // Get transaction receiver safely
        let receiver = match get_transaction_receiver() {
            Some(rx) => rx,
            None => {
                warn!("Transaction receiver not initialized");
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        // Try to receive a transaction with timeout to avoid blocking
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(transaction) => {
                info!(
                    "{} Processing transaction: {}",
                    Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string().blue(),
                    transaction.hash.green()
                );

                // Execute transaction in VM - limit mutex lock scope
                let execution_result = match process_transaction_in_vm(&transaction) {
                    Ok(result) => result,
                    Err(e) => {
                        error!("Failed to process transaction: {}", e);
                        continue;
                    }
                };

                // Process execution result
                match execution_result {
                    TransactionStatus::Success { gas_used, changes } => {
                        info!(
                            "{} Transaction executed successfully. Gas used: {}",
                            Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string().blue(),
                            gas_used
                        );

                        // Apply state changes safely
                        if let Err(e) = apply_state_changes(&changes) {
                            error!("Failed to apply state changes: {}", e);
                            continue;
                        }

                        // Add transaction to pending pool for block inclusion
                        match get_mutex_lock(&PENDING_TRANSACTIONS, "add_successful_transaction") {
                            Ok(mut pending) => pending.push(transaction),
                            Err(e) => error!("Failed to add successful transaction to pending pool: {}", e)
                        }
                    }
                    TransactionStatus::Failed { error, gas_used } => {
                        warn!(
                            "{} Transaction failed: {:?}. Gas used: {}",
                            Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string().blue(),
                            error,
                            gas_used
                        );

                        // Charge gas even for failed transactions
                        if let Err(e) = apply_gas_charge(&transaction.sender, gas_used * transaction.gas_price) {
                            error!("Failed to charge gas for failed transaction: {}", e);
                        }
                    }
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                // This is normal, just continue
            },
            Err(RecvTimeoutError::Disconnected) => {
                warn!("Transaction channel disconnected");
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}