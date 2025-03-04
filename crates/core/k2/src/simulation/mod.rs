use chrono::Local;
use colored::*;
use consensus_pos::Blake3Algorithm;
use crossbeam::channel::{unbounded, Receiver, Sender};
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::block::Block;
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN, TOTAL_TOKENS};
use crate::transaction::{Transaction, TransactionType};
use lazy_static::lazy_static;

use mona_types::address::Address;
use mona_types::gas::GasSchedule;
use mona_vm::{MonaVM, TransactionContext, TransactionStatus};

// Define the Sender and Receiver separately
pub static mut TRANSACTION_SENDER: Option<Sender<Transaction>> = None;
pub static mut TRANSACTION_RECEIVER: Option<Receiver<Transaction>> = None;

// Add a VM instance for transaction execution
lazy_static! {
    static ref PENDING_TRANSACTIONS: Mutex<Vec<Transaction>> = Mutex::new(Vec::new());
    static ref VM: Mutex<MonaVM> = Mutex::new(MonaVM::new());
}

pub fn run_blockchain(running: Arc<Mutex<bool>>, address: String) {
    let max_tokens = 11_000_000; // Maximum token supply
    let mut tokens_per_block = 25; // Initial block reward
    let halving_interval = 210_000; // Halve the block reward every 210,000 blocks
    let block_size = 2_250_000; // 2.25 MB in bytes

    // Assume there's a global variable for pending transactions
    static PENDING_TRANSACTIONS: Mutex<Vec<Transaction>> = Mutex::new(Vec::new());

    unsafe {
        // Initialize the channel within the function
        let (sender, receiver) = unbounded();
        TRANSACTION_SENDER = Some(sender);
        TRANSACTION_RECEIVER = Some(receiver);

        if BLOCKCHAIN.is_empty() {
            let genesis_data = vec![0; block_size];
            let genesis_transactions = vec![];
            let hasher = Blake3Algorithm;
            BLOCKCHAIN.push_back(Block::new(
                0,
                genesis_data,
                String::from("0"),
                tokens_per_block,
                genesis_transactions,
                address.clone(),
                hasher,
            ));
            TOTAL_TOKENS += tokens_per_block;
            BALANCES
                .lock()
                .unwrap()
                .entry(address.clone())
                .and_modify(|balance| *balance += tokens_per_block)
                .or_insert(tokens_per_block);
            info!(
                "Genesis block created with hash: {}",
                BLOCKCHAIN.back().unwrap().hash
            );
        }

        loop {
            // Receive transactions from the channel
            if let Ok(transaction) = TRANSACTION_RECEIVER.as_ref().unwrap().try_recv() {
                info!("Received new transaction: {:?}", transaction);
                PENDING_TRANSACTIONS.lock().unwrap().push(transaction);
            }

            let _running = running.lock().unwrap();

            if TOTAL_TOKENS >= max_tokens {
                warn!("Reached maximum token supply. Only processing transactions.");
                tokens_per_block = 0; // Set block reward to 0
            }

            // Calculate miner reward based on token supply
            let reward = if TOTAL_TOKENS < max_tokens {
                tokens_per_block
            } else {
                0
            };

            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];

            let mut transactions = vec![];

            // Move the clearing of pending transactions after they are processed
            {
                let mut pending = PENDING_TRANSACTIONS.lock().unwrap();
                transactions.append(&mut pending);
            }

            // If there are no transactions, create a zero-fee transaction for the miner
            if transactions.is_empty() {
                transactions.push(Transaction {
                    sender: "system".to_string(),
                    receiver: address.clone(),
                    amount: 0,
                    gas_cost: GasSchedule::default().contract_execution_base_cost as f64,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    signature: None, // Add an empty signature or a valid one if available
                    tx_type: TransactionType::Transfer, // Add default transfer type
                    data: vec![],    // Add empty data
                    coin_type: Some("KARI".to_string()),
                    hash: "".to_string(),
                    gas_limit: 1000000,
                    gas_price: 1,
                    nonce: 0, // Add coin type if available
                });
                info!("No transactions found. Created a zero-fee transaction.");
            }

            let hasher = Blake3Algorithm;
            let new_block = Block::new(
                prev_block.index + 1,
                new_data,
                prev_block.hash.clone(),
                reward, // Use calculated reward
                transactions.clone(),
                address.clone(),
                hasher,
            );

            if !new_block.verify(prev_block) {
                error!("Block verification failed!");
                break;
            }

            BLOCKCHAIN.push_back(new_block.clone());

            // Update balances for each transaction
            for tx in transactions.iter() {
                let mut balances = BALANCES.lock().unwrap();
                if let Some(sender_balance) = balances.get_mut(&tx.sender) {
                    // Check if sender has sufficient funds before subtracting
                    let total_cost = tx.amount + tx.gas_cost as u64;
                    if *sender_balance >= total_cost {
                        *sender_balance -= total_cost;
                        // Only credit the receiver if the sender had sufficient funds
                        *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                    } else {
                        // Log failed transaction due to insufficient funds
                        error!("Transaction failed: insufficient funds for tx: {:?}", tx);
                        // Skip this transaction - don't update balances
                        continue;
                    }
                } else {
                    // Sender address doesn't exist in balances
                    error!("Transaction failed: sender address not found for tx: {:?}", tx);
                    continue;
                }
            }
            
            // Move this outside the transaction loop so it only processes valid transactions
            let transaction_fees: u64 = new_block
                .transactions
                .iter()
                .filter(|tx| {
                    // Only include fees from transactions where sender had sufficient funds
                    if let Some(balance) = BALANCES.lock().unwrap().get(&tx.sender) {
                        *balance >= tx.amount + tx.gas_cost as u64
                    } else {
                        false
                    }
                })
                .map(|tx| tx.gas_cost as u64)
                .sum();
            
            BALANCES
                .lock()
                .unwrap()
                .entry(address.clone())
                .and_modify(|balance| *balance += transaction_fees + reward)
                .or_insert(transaction_fees + reward);

            // Update TOTAL_TOKENS only if it's less than the max supply
            if TOTAL_TOKENS < max_tokens {
                TOTAL_TOKENS += tokens_per_block;
            }

            // Save blockchain every time a new block is created
            save_blockchain();

            println!(
                "{} {} | block={} | hash={:}... | prev={:}... | miner={} | reward={}",
                "[INFO]".green(),
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                BLOCKCHAIN.len().to_string().blue(),
                new_block.hash[..48].yellow(),
                new_block.prev_hash[..42].yellow(),
                format!("{}t", transaction_fees).cyan(),
                format!("{}t", tokens_per_block).cyan()
            );

            if BLOCKCHAIN.len() % halving_interval == 0 && TOTAL_TOKENS < max_tokens {
                tokens_per_block /= 2;
                println!(
                    "{} Block reward halved to {}",
                    "[HALV]".red(),
                    format!("{} tokens", tokens_per_block).red()
                );
            }

            println!(
                "{} blocks={} supply={}",
                "[STAT]".magenta(),
                BLOCKCHAIN.len().to_string().blue(),
                TOTAL_TOKENS.to_string().magenta()
            );
            println!();

            thread::sleep(Duration::from_millis(550));
        }
    }
}

pub fn process_transactions(running: Arc<Mutex<bool>>) {
    let mut vm = VM.lock().unwrap();

    while *running.lock().unwrap() {
        // Retrieve transaction from channel
        if let Ok(transaction) = unsafe {
            TRANSACTION_RECEIVER
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_millis(100))
        } {
            info!(
                "{} Processing transaction: {}",
                Local::now()
                    .format("[%Y-%m-%d %H:%M:%S]")
                    .to_string()
                    .blue(),
                transaction.hash.green()
            );

            // Convert transaction to VM format
            let tx_bytes = transaction.serialize();
            let sender_address =
                Address::from_hex(&transaction.sender).expect("Invalid sender address");

            // Create transaction context
            let context = TransactionContext {
                max_gas_units: transaction.gas_limit,
                gas_unit_price: transaction.gas_price,
                sender: sender_address,
                sequence_number: transaction.nonce,
                expiration_timestamp_secs: transaction.timestamp,
            };

            // Execute transaction in VM
            match vm.execute_transaction(tx_bytes, context) {
                TransactionStatus::Success { gas_used, changes } => {
                    info!(
                        "{} Transaction executed successfully. Gas used: {}",
                        Local::now()
                            .format("[%Y-%m-%d %H:%M:%S]")
                            .to_string()
                            .blue(),
                        gas_used
                    );

                    // Apply state changes
                    apply_state_changes(&changes);

                    // Add transaction to pending pool for block inclusion
                    let mut pending = PENDING_TRANSACTIONS.lock().unwrap();
                    pending.push(transaction.clone());
                }
                TransactionStatus::Failed { error, gas_used } => {
                    warn!(
                        "{} Transaction failed: {:?}. Gas used: {}",
                        Local::now()
                            .format("[%Y-%m-%d %H:%M:%S]")
                            .to_string()
                            .blue(),
                        error,
                        gas_used
                    );

                    // Charge gas even for failed transactions
                    apply_gas_charge(&transaction.sender, gas_used * transaction.gas_price);
                }
            }
        }
    }
}

// Helper function to apply VM state changes to the blockchain
pub fn apply_state_changes(changes: &mona_vm::ChangeSet) {
    // Direct access to BALANCES, no need for Option handling
    let mut balances = BALANCES.lock().unwrap();

    // Since writes and deletes are private, you need to use accessor methods
    // You'll need to modify the ChangeSet struct to provide getters or iterators

    // For example, if you add methods like:
    for (key, value) in changes.get_writes() {
        // Assuming you add this method
        if key.len() == 32 {
            let address = hex::encode(key);
            balances.insert(address, u64::from_le_bytes(value[..8].try_into().unwrap()));
        }
    }

    for key in changes.get_deletes() {
        // Assuming you add this method
        if key.len() == 32 {
            let address = hex::encode(key);
            balances.remove(&address);
        }
    }
}

// Helper function to charge gas for failed transactions
pub fn apply_gas_charge(sender: &str, gas_charge: u64) {
    let mut balances = BALANCES.lock().unwrap();

    if let Some(balance) = balances.get_mut(sender) {
        if *balance >= gas_charge {
            *balance -= gas_charge;
        }
    }
}
