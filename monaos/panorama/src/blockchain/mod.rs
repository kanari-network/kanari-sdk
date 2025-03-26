use crate::block::{Block, Transaction};
use bincode;
use consensus_pos::Blake3Algorithm;
use dirs;
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};
use log::{info, warn, error, debug};

use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::ptr::addr_of;
use std::sync::Mutex;

// Define constants for Kari token
pub const KARI_DECIMALS: u8 = 9;
pub const KARI_BASE: u64 = 1_000_000_000;  // 10^9
pub const INITIAL_KARI_SUPPLY: u64 = 100_000_000 * KARI_BASE;  // 100 million kari
pub const TRANSACTION_FEE_PERCENT: u64 = 1;  // 1% transaction fee

// Define global variables for the blockchain
pub static mut TOTAL_TOKENS: u64 = 0;
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();

lazy_static! {
    pub static ref BALANCES: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

pub fn load_blockchain_with_retry() -> Result<(), StorageError> {
    // First attempt - simple load
    let result = load_blockchain();
    if result.is_ok() {
        return result;
    }
    
    // Second attempt after cleanup
    let result = load_blockchain();
    if result.is_ok() {
        return result;
    }
    
    // One more attempt with delay
    std::thread::sleep(std::time::Duration::from_millis(500));
    load_blockchain()
}

pub fn save_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;

    unsafe {
        let data = bincode::serialize(addr_of!(BLOCKCHAIN).as_ref().unwrap())?;
        storage.save_data(b"blockchain", &data)?;
        storage.flush()?;
    }

    Ok(())
}

pub fn init_blockchain_state() {
    unsafe {
        // Initialize with empty collections
        if BLOCKCHAIN.is_empty() {
            BLOCKCHAIN = VecDeque::new();
        }
        
        // Initialize total tokens to 0 (will be set properly when loading blockchain)
        TOTAL_TOKENS = 0;
        
        // Make sure we have a valid balances mutex
        let balances = BALANCES.lock().unwrap();
        debug!("Initialized blockchain state with {} balances", balances.len());
    }
}

#[derive(Debug)]
pub enum BlockchainError {
    Storage(StorageError),
    Balance(String),
    Initialization(String)
}

impl From<StorageError> for BlockchainError {
    fn from(error: StorageError) -> Self {
        BlockchainError::Storage(error)
    }
}

impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BlockchainError::Storage(e) => write!(f, "Storage error: {}", e),
            BlockchainError::Balance(e) => write!(f, "Balance error: {}", e),
            BlockchainError::Initialization(e) => write!(f, "Initialization error: {}", e),
        }
    }
}

pub fn get_balance(address: &str) -> Result<u64, BlockchainError> {
    let max_retries = 3;
    let mut attempts = 0;

    while attempts < max_retries {
        // BALANCES is now directly a Mutex, not an Option<Mutex>
        match BALANCES.lock() {
            Ok(guard) => {
                return Ok(*guard.get(address).unwrap_or(&0));
            }
            Err(_) => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
        }
    }

    // Add error handling for the case where all attempts fail
    Err(BlockchainError::Balance(
        "Failed to acquire balance lock after multiple attempts".into(),
    ))
}

// Calculate transaction fee
pub fn calculate_fee(amount: u64) -> u64 {
    amount * TRANSACTION_FEE_PERCENT / 100
}

// Calculate net amount after fee deduction
pub fn calculate_net_amount(amount: u64) -> u64 {
    amount - calculate_fee(amount)
}

pub fn load_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;
    init_blockchain_state();

    match storage.load_data(b"blockchain")? {
        Some(value) => {
            unsafe { BLOCKCHAIN = bincode::deserialize(&value)? };
            // Calculate balances and total tokens
            let mut balances = HashMap::new();
            let mut total_tokens = 0;

            for block in unsafe { BLOCKCHAIN.iter() } {
                // Add tokens from block to total supply and miner's balance
                total_tokens += block.tokens;
                *balances.entry(block.address.clone()).or_insert(0) += block.tokens;

                // Process all transactions including fees
                for tx in &block.transactions {
                    // Calculate fee
                    let fee = calculate_fee(tx.amount);
                    let net_amount = tx.amount - fee;
                    
                    // Deduct full amount from sender
                    *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
                    
                    // Add net amount to receiver
                    *balances.entry(tx.receiver.clone()).or_insert(0) += net_amount;
                    
                    // Add fee to block miner
                    *balances.entry(block.address.clone()).or_insert(0) += fee;
                }
            }

            *BALANCES.lock().unwrap() = balances;
            unsafe { TOTAL_TOKENS = total_tokens };

            info!("Blockchain loaded successfully with {} blocks", unsafe { BLOCKCHAIN.len() });
            info!("Total supply: {} kari", total_tokens / KARI_BASE);
        }
        None => {
            info!("No blockchain data found, initializing new chain");
            unsafe { BLOCKCHAIN = VecDeque::new() };
            *BALANCES.lock().unwrap() = HashMap::new();
            unsafe { TOTAL_TOKENS = 0 };
        }
    }

    storage.flush()?;
    Ok(())
}

// Validate if a transaction can be processed (sender has sufficient funds)
pub fn validate_transaction(tx: &Transaction) -> bool {
    if let Ok(balances) = BALANCES.lock() {
        let sender_balance = *balances.get(&tx.sender).unwrap_or(&0);
        
        // Calculate total cost including fee
        let fee = calculate_fee(tx.amount);
        let total_cost = tx.amount + fee;
        
        return sender_balance >= total_cost;
    }
    false
}