use crate::block::Block;
use bincode;
use consensus_pos::Blake3Algorithm;
use dirs;
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};


use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::ptr::addr_of;
use std::sync::Mutex;

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
        // No need to check BALANCES.is_none() anymore - it's initialized by lazy_static
        // You can still initialize it with default values if needed
        let balances = BALANCES.lock().unwrap();
        if balances.is_empty() {
            // Add any initial balances here if needed
            // For example: balances.insert("genesis_address".to_string(), 1000000);
        }
        
        if BLOCKCHAIN.is_empty() {
            BLOCKCHAIN = VecDeque::new();
        }
        TOTAL_TOKENS = 0;
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
                total_tokens += block.tokens;
                *balances.entry(block.address.clone()).or_insert(0) += block.tokens;

                for tx in &block.transactions {
                    *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
                    *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                }
            }

            *BALANCES.lock().unwrap() = balances;
            unsafe { TOTAL_TOKENS = total_tokens };

            println!("Blockchain loaded successfully");
        }
        None => {
            println!("No blockchain data found, initializing new chain");
            unsafe { BLOCKCHAIN = VecDeque::new() };
            *BALANCES.lock().unwrap() = HashMap::new();
            unsafe { TOTAL_TOKENS = 0 };
        }
    }

    storage.flush()?;
    Ok(())
}