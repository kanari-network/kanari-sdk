use chrono::Local;
use colored::*;
use consensus_pos::Blake3Algorithm;
use crossbeam::channel::{unbounded, Receiver, Sender, RecvTimeoutError};
use log::{error, info, warn, debug};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::block::Block;
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN, TOTAL_TOKENS};
use crate::transaction::{Transaction, TransactionType};
use lazy_static::lazy_static;

use mona_types::gas::GasSchedule;
use mona_vm::MonaVM;
// Use Once to safely initialize channels 
use std::sync::Once;

pub mod process_block_transactions;
pub mod process_transactions;
pub mod process_transaction_in_vm;
pub mod apply_gas_charge;
pub mod apply_state_changes;

use crate::simulation::process_block_transactions::process_block_transactions;

static INIT: Once = Once::new();

// Custom error types for better error handling
#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("Lock acquisition failed: {0}")]
    LockError(String),
    
    #[error("Address conversion error: {0}")]
    AddressError(String),
    
    #[error("Insufficient funds: required {0}, available {1}")]
    InsufficientFunds(u64, u64),
    
    #[error("Address not found: {0}")]
    AddressNotFound(String),
    
    #[error("Transaction execution failed: {0}")]
    TransactionError(String),
    
    #[error("State update error: {0}")]
    StateError(String),
}

// Use Arc<Mutex<>> instead of static mut for thread safety
lazy_static! {
    static ref TRANSACTION_CHANNEL: Arc<Mutex<Option<(Sender<Transaction>, Receiver<Transaction>)>>> = 
        Arc::new(Mutex::new(None));
    static ref PENDING_TRANSACTIONS: Mutex<Vec<Transaction>> = Mutex::new(Vec::new());
    static ref VM: Mutex<MonaVM> = Mutex::new(MonaVM::new());
}

// Helper function to safely get mutex locks with timeout to prevent deadlocks
fn get_mutex_lock<'a, T>(mutex: &'a Mutex<T>, operation: &str) -> Result<MutexGuard<'a, T>, SimulationError> {
    mutex.lock().map_err(|e| {
        error!("Failed to acquire lock for {}: {:?}", operation, e);
        SimulationError::LockError(format!("{}: {:?}", operation, e))
    })
}

// Initialize transaction channels safely
fn init_transaction_channel() {
    INIT.call_once(|| {
        let (sender, receiver) = unbounded();
        match get_mutex_lock(&TRANSACTION_CHANNEL, "init_transaction_channel") {
            Ok(mut channel) => *channel = Some((sender, receiver)),
            Err(e) => error!("Failed to initialize transaction channel: {}", e),
        }
    });
}

// Safe getters for transaction sender and receiver
pub fn get_transaction_sender() -> Option<Sender<Transaction>> {
    match TRANSACTION_CHANNEL.try_lock() {
        Ok(channel) => channel.as_ref().map(|(sender, _)| sender.clone()),
        Err(e) => {
            debug!("Could not acquire lock for transaction sender: {:?}", e);
            None
        }
    }
}

pub fn get_transaction_receiver() -> Option<Receiver<Transaction>> {
    match TRANSACTION_CHANNEL.try_lock() {
        Ok(channel) => channel.as_ref().map(|(_, receiver)| receiver.clone()),
        Err(e) => {
            debug!("Could not acquire lock for transaction receiver: {:?}", e);
            None
        }
    }
}

// Helper function to get blockchain length safely without unsafe
fn get_blockchain_len() -> usize {
    // Use a read-only atomic operation if possible in the actual implementation
    // For now, we'll continue using unsafe but wrap it in a function
    unsafe { BLOCKCHAIN.len() }
}

// Helper function to create a blockchain block wrapper to reduce unsafe usage
fn create_block(
    index: u32, 
    data: Vec<u8>, 
    prev_hash: String, 
    tokens: u64, 
    transactions: Vec<Transaction>, 
    address: String
) -> Block<Blake3Algorithm> {
    Block::new(
        index,
        data,
        prev_hash,
        tokens,
        transactions,
        address,
        Blake3Algorithm,
    )
}

pub fn run_blockchain(running: Arc<Mutex<bool>>, address: String) {
    // Initialize transaction channels
    init_transaction_channel();

    let max_tokens = 100_000_000; // Maximum token supply
    let mut tokens_per_block = 25; // Initial block reward
    let halving_interval = 2_010_000; // Halve the block reward every 210,000 blocks
    let block_size = 1_250_000; // 1.25 MB in bytes

    // Create genesis block if blockchain is empty
    {
        let blockchain_len = get_blockchain_len();
        if blockchain_len == 0 {
            let genesis_data = vec![0; block_size];
            let genesis_transactions = vec![];
            
            let genesis_block = create_block(
                0,
                genesis_data,
                String::from("0"),
                tokens_per_block,
                genesis_transactions,
                address.clone()
            );
            
            unsafe {
                BLOCKCHAIN.push_back(genesis_block);
                TOTAL_TOKENS += tokens_per_block;
                
                // Update miner balance for genesis reward
                match get_mutex_lock(&BALANCES, "genesis_balance_update") {
                    Ok(mut balances) => {
                        balances.entry(address.clone())
                            .and_modify(|balance| *balance += tokens_per_block)
                            .or_insert(tokens_per_block);
                            
                        info!(
                            "Genesis block created with hash: {}",
                            BLOCKCHAIN.back().unwrap().hash
                        );
                    },
                    Err(e) => error!("Failed to update genesis miner balance: {}", e),
                }
            }
        }
    }

    // Main blockchain loop
    loop {
        // Check if we should continue running
        if !*running.lock().unwrap() {
            break;
        }

        // Process incoming transactions - limit lock scope
        if let Some(receiver) = get_transaction_receiver() {
            while let Ok(transaction) = receiver.try_recv() {
                info!("Received new transaction: {}", transaction.hash);
                debug!("Transaction details: {:?}", transaction);
                
                match get_mutex_lock(&PENDING_TRANSACTIONS, "add_pending_transaction") {
                    Ok(mut pending) => pending.push(transaction),
                    Err(e) => error!("Failed to add transaction to pending pool: {}", e),
                }
            }
        }

        // Calculate miner reward based on token supply
        let reward = if unsafe { TOTAL_TOKENS } < max_tokens {
            tokens_per_block
        } else {
            if tokens_per_block > 0 {
                warn!("Reached maximum token supply. Only processing transactions.");
                tokens_per_block = 0; // Set block reward to 0
            }
            0
        };

        // Safely get pending transactions
        let transactions = {
            match get_mutex_lock(&PENDING_TRANSACTIONS, "get_pending_transactions") {
                Ok(mut pending) => {
                    if pending.is_empty() {
                        // Create zero-fee transaction only if there are no pending transactions
                        vec![Transaction {
                            sender: "system".to_string(),
                            receiver: address.clone(),
                            amount: 0,
                            gas_cost: GasSchedule::default().contract_execution_base_cost as f64,
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_secs(),
                            signature: None,
                            tx_type: TransactionType::Transfer,
                            data: vec![],
                            coin_type: Some("KARI".to_string()),
                            hash: "".to_string(),
                            gas_limit: 1000000,
                            gas_price: 1,
                            nonce: 0,
                        }]
                    } else {
                        // Take all pending transactions
                        std::mem::take(&mut *pending)
                    }
                },
                Err(e) => {
                    error!("Failed to get pending transactions: {}", e);
                    vec![] // Return empty transactions array on error
                }
            }
        };

        // Create new block
        let (prev_block, new_block) = unsafe {
            let prev_block = BLOCKCHAIN.back().unwrap();
            let new_data = vec![0; block_size];
            
            let new_block = create_block(
                prev_block.index + 1,
                new_data,
                prev_block.hash.clone(),
                reward,
                transactions,
                address.clone()
            );
            
            (prev_block.clone(), new_block)
        };

        // Verify block before adding to chain
        if !new_block.verify(&prev_block) {
            error!("Block verification failed!");
            break;
        }

        // Process transactions and update balances - collect valid transactions and failed ones
        match process_block_transactions(&new_block) {
            Ok((valid_transactions, transaction_fees)) => {
                // Only add block to chain if it has valid transactions or is a reward-only block
                if !valid_transactions.is_empty() || reward > 0 {
                    unsafe {
                        // Update blockchain with the new block that has filtered transactions
                        let mut updated_block = new_block;
                        updated_block.transactions = valid_transactions;
                        BLOCKCHAIN.push_back(updated_block.clone());
                        
                        // Update miner's balance with transaction fees and block reward
                        match get_mutex_lock(&BALANCES, "update_miner_balance") {
                            Ok(mut balances) => {
                                balances.entry(address.clone())
                                    .and_modify(|balance| *balance += transaction_fees + reward)
                                    .or_insert(transaction_fees + reward);
                            },
                            Err(e) => error!("Failed to update miner balance: {}", e),
                        }

                        // Update total tokens only if it won't exceed max supply
                        if TOTAL_TOKENS < max_tokens {
                            TOTAL_TOKENS += reward;
                        }

                        // Save blockchain state
                        save_blockchain();

                        println!(
                            "{} {} | block={} | hash={:}... | prev={:}... | miner={} | reward={}",
                            "[INFO]".green(),
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            BLOCKCHAIN.len().to_string().blue(),
                            updated_block.hash[..48.min(updated_block.hash.len())].yellow(),
                            updated_block.prev_hash[..42.min(updated_block.prev_hash.len())].yellow(),
                            format!("{}t", transaction_fees).cyan(),
                            format!("{}t", reward).cyan()
                        );

                        // Check for halving
                        if BLOCKCHAIN.len() % halving_interval == 0 && TOTAL_TOKENS < max_tokens && tokens_per_block > 0 {
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
                    }
                }
            },
            Err(e) => {
                error!("Failed to process block transactions: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(550));
    }
}
