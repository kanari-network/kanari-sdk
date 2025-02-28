use crate::block::Block;
use mona_types::gas::GasSchedule;
use crate::transaction::{Transaction, TransactionType};
use bincode;
use consensus_pos::Blake3Algorithm;
use dirs;
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};
use mona_storage::{FileStorage, StorageError2};
use std::path::Path;

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::ptr::addr_of;
use std::sync::Mutex;

// Define global variables for the blockchain
pub static mut BALANCES: Option<Mutex<HashMap<String, u64>>> = None;
pub static mut TOTAL_TOKENS: u64 = 0;
pub static mut BLOCKCHAIN: VecDeque<Block<Blake3Algorithm>> = VecDeque::new();

pub fn get_kari_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Unable to find home directory");
    path.push(".kari");
    fs::create_dir_all(&path).expect("Unable to create .kari directory");
    path
}

fn cleanup_db_locks() -> std::io::Result<()> {
    let lock_path = get_kari_dir().join("blockchain_db").join("LOCK");
    if lock_path.exists() {
        fs::remove_file(lock_path)?;
    }
    Ok(())
}

pub fn save_blockchain() -> Result<(), StorageError> {
    let _ = cleanup_db_locks();
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
        if BALANCES.is_none() {
            BALANCES = Some(Mutex::new(HashMap::new()));
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
    Initialization(String),
    FileStorage(StorageError2),
}

impl From<StorageError> for BlockchainError {
    fn from(error: StorageError) -> Self {
        BlockchainError::Storage(error)
    }
}

impl From<StorageError2> for BlockchainError {
    fn from(error: StorageError2) -> Self {
        BlockchainError::FileStorage(error)
    }
}


// Add file storage function
pub fn store_file(file_path: &Path) -> Result<String, BlockchainError> {
    let filename = file_path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();

    let storage = FileStorage::upload(file_path, filename)?;
    let file_id = storage.id.to_string();
    
    // Create file storage transaction
    let transaction = Transaction {
        sender: "system".to_string(),
        receiver: file_id.clone(),
        amount: 0,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
            gas_cost: GasSchedule::default().contract_execution_base_cost as f64,
        signature: None,
        tx_type: TransactionType::FileStore,
        data: storage.path.to_str().unwrap_or("").as_bytes().to_vec(),
        coin_type: None
    };

    // Add transaction to current block
    unsafe {
        if let Some(last_block) = BLOCKCHAIN.back_mut() {
            last_block.transactions.push(transaction);
            save_blockchain()?;
        }
    }

    Ok(file_id)
}


impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BlockchainError::Storage(e) => write!(f, "Storage error: {}", e),
            BlockchainError::Balance(e) => write!(f, "Balance error: {}", e),
            BlockchainError::Initialization(e) => write!(f, "Initialization error: {}", e),
            BlockchainError::FileStorage(storage_error2) => write!(f, "File storage error: {}", storage_error2),
        }
    }
}

pub fn get_balance(address: &str) -> Result<u64, BlockchainError> {
    let max_retries = 3;
    let mut attempts = 0;

    while attempts < max_retries {
        unsafe {
            if let Some(balances) = &BALANCES {
                match balances.lock() {
                    Ok(guard) => {
                        return Ok(*guard.get(address).unwrap_or(&0));
                    }
                    Err(_) => {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                }
            } else {
                init_blockchain_state();
                load_blockchain()?;
            }
        }
        attempts += 1;
    }

    Err(BlockchainError::Balance(
        "Failed to get balance after retries".to_string(),
    ))
}

pub fn transfer_coins(sender: String, receiver: String, amount: u64) -> Result<(), BlockchainError> {
    let max_retries = 3;
    let mut attempts = 0;

    while attempts < max_retries {
        // Clean up any stale locks first
        let _ = cleanup_db_locks();
        
        unsafe {
            if let Some(balances) = &BALANCES {
                match balances.lock() {
                    Ok(mut guard) => {
                        // Check sender balance
                        let sender_balance = *guard.get(&sender).unwrap_or(&0);
                        if sender_balance < amount {
                            return Err(BlockchainError::Balance("Insufficient balance".to_string()));
                        }

                        // Update balances
                        *guard.entry(sender.clone()).or_insert(0) -= amount;
                        *guard.entry(receiver.clone()).or_insert(0) += amount;

                        // Create transaction
                        let transaction = Transaction {
                            sender: sender.clone(),
                            receiver: receiver.clone(),
                            amount,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                                gas_cost: GasSchedule::default().contract_execution_base_cost as f64,
                            signature: None,    // Add an empty signature or a valid one if available
                            tx_type: TransactionType::Transfer,
                            data: vec![],      // No additional data for basic transfer
                            coin_type: Some("KARI".to_string())
                        };

                        // Add to blockchain and save
                        if let Some(last_block) = BLOCKCHAIN.back_mut() {
                            last_block.transactions.push(transaction);
                            drop(guard); // Release lock before saving
                            save_blockchain()?;
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        continue;
                    }
                }
            } else {
                init_blockchain_state();
                load_blockchain()?;
            }
        }
        attempts += 1;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err(BlockchainError::Balance("Failed to transfer coins after retries".to_string()))
}

pub fn load_blockchain() -> Result<(), StorageError> {
    let _ = cleanup_db_locks();
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;
    init_blockchain_state();

    unsafe {
        match storage.load_data(b"blockchain")? {
            Some(value) => {
                BLOCKCHAIN = bincode::deserialize(&value)?;

                // Calculate balances and total tokens
                let mut balances = HashMap::new();
                let mut total_tokens = 0;

                for block in BLOCKCHAIN.iter() {
                    total_tokens += block.tokens;
                    *balances.entry(block.address.clone()).or_insert(0) += block.tokens;

                    for tx in &block.transactions {
                        *balances.entry(tx.sender.clone()).or_insert(0) -= tx.amount;
                        *balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;
                    }
                }

                BALANCES = Some(Mutex::new(balances));
                TOTAL_TOKENS = total_tokens;

                println!("Blockchain loaded successfully");
            }
            None => {
                println!("No blockchain data found, initializing new chain");
                BLOCKCHAIN = VecDeque::new();
                BALANCES = Some(Mutex::new(HashMap::new()));
                TOTAL_TOKENS = 0;
            }
        }
    }

    storage.flush()?;
    Ok(())
}
