use crate::block::{Block, Transaction};
use bincode;
use consensus_pos::Blake3Algorithm;
// Replace with common import
use common::get_kari_dir;
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};
// use crate::block::Transaction;
// use std::time::{SystemTime, UNIX_EPOCH};

// // Import key directly for signing functions
// use key::{sign_message, verify_signature};

use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, RwLock, atomic::{AtomicU64, Ordering}};

// Define improved thread-safe blockchain globals
lazy_static! {
    pub static ref BLOCKCHAIN_DATA: BlockchainData = BlockchainData::new();
    pub static ref BALANCES: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

/// Improved blockchain data container with thread-safety and performance features
pub struct BlockchainData {
    chain: RwLock<VecDeque<Block<Blake3Algorithm>>>,
    total_tokens: AtomicU64,
    // Can be extended with cache and other performance features
    block_height_cache: RwLock<HashMap<String, usize>>, // Hash -> Height mapping
}

impl BlockchainData {
    pub fn new() -> Self {
        BlockchainData {
            chain: RwLock::new(VecDeque::new()),
            total_tokens: AtomicU64::new(0),
            block_height_cache: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn get_total_tokens(&self) -> u64 {
        self.total_tokens.load(Ordering::Relaxed)
    }
    
    pub fn add_tokens(&self, amount: u64) {
        self.total_tokens.fetch_add(amount, Ordering::Relaxed);
    }
    
    pub fn get_block(&self, index: usize) -> Option<Block<Blake3Algorithm>> {
        self.chain.read().unwrap().get(index).cloned()
    }
    
    pub fn add_block(&self, block: Block<Blake3Algorithm>) {
        let mut chain = self.chain.write().unwrap();
        let height = chain.len();
        
        // Update token count
        self.total_tokens.fetch_add(block.tokens, Ordering::Relaxed);
        
        // Update cache
        self.block_height_cache.write().unwrap().insert(block.hash.clone(), height);
        
        // Add block to chain
        chain.push_back(block);
    }
    
    pub fn len(&self) -> usize {
        self.chain.read().unwrap().len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.chain.read().unwrap().is_empty()
    }
    
    pub fn iter(&self) -> Vec<Block<Blake3Algorithm>> {
        self.chain.read().unwrap().iter().cloned().collect()
    }
}

// Helper functions to maintain backward compatibility

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

// Improved save function that ensures balances are saved
pub fn save_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;

    // Save blockchain data
    let data = bincode::serialize(&BLOCKCHAIN_DATA.chain.read().unwrap().clone())?;
    storage.save_data(b"blockchain", &data)?;

    // Save balances separately for better reliability
    let balances = BALANCES.lock().unwrap().clone();
    let balances_data = bincode::serialize(&balances)?;
    storage.save_data(b"balances", &balances_data)?;
    
    storage.flush()?;
    log::debug!("Blockchain and balances saved successfully");
    
    Ok(())
}

pub fn init_blockchain_state() {
    // BALANCES already initialized by lazy_static
    let balances = BALANCES.lock().unwrap();
    if balances.is_empty() {
        // Add any initial balances here if needed
    }
    
    // No need to initialize BLOCKCHAIN_DATA as it's created by lazy_static
}

#[derive(Debug)]
pub enum BlockchainError {
    Storage(StorageError),
    Balance(String),
    Initialization(String),
    Transaction(String),
    InsufficientFunds(String),
    InvalidAddress(String)
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
            BlockchainError::Transaction(e) => write!(f, "Transaction error: {}", e),
            BlockchainError::InsufficientFunds(e) => write!(f, "Insufficient funds: {}", e),
            BlockchainError::InvalidAddress(e) => write!(f, "Invalid address: {}", e),
        }
    }
}

// Helper function to normalize addresses
pub fn normalize_address(address: &str) -> String {
    if !address.starts_with("0x") {
        format!("0x{}", address)
    } else {
        address.to_string()
    }
}

/// Transfer tokens from one address to another
pub fn transfer_tokens(
    from_address: &str,
    to_address: &str,
    amount: u64,
) -> Result<Transaction, BlockchainError> {
    // Validate addresses
    if from_address.trim().is_empty() || to_address.trim().is_empty() {
        return Err(BlockchainError::InvalidAddress("Empty address provided".to_string()));
    }
    
    if amount == 0 {
        return Err(BlockchainError::Transaction("Cannot transfer zero tokens".to_string()));
    }
    
    // Normalize addresses for consistent handling
    let from = normalize_address(from_address);
    let to = normalize_address(to_address);
    
    // Validate addresses are different
    if from == to {
        return Err(BlockchainError::Transaction("Cannot transfer to same address".to_string()));
    }
    
    // Check sender's balance
    let balance = get_balance(&from)?;
    if balance < amount {
        return Err(BlockchainError::InsufficientFunds(
            format!("Address {} has {} tokens, tried to send {}", from, balance, amount)
        ));
    }
    
    // Update balances
    let mut balances = match BALANCES.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(BlockchainError::Transaction("Failed to lock balances".to_string())),
    };
    
    // Deduct from sender
    *balances.entry(from.clone()).or_insert(0) -= amount;
    
    // Add to receiver
    *balances.entry(to.clone()).or_insert(0) += amount;
    
    // Create transaction
    let transaction = Transaction {
        sender: from.clone(),
        receiver: to.clone(),
        amount,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        signature: None, // In a real implementation we would sign the transaction
    };
    
    // Save balances immediately to persist changes
    drop(balances); // Release lock before saving
    
    // Save blockchain state to persist the balance changes
    if let Err(e) = save_blockchain() {
        log::error!("Failed to save blockchain after transfer: {}", e);
        // Consider reverting the balance changes if save fails
        // But be careful about concurrency issues
    }
    
    // Log the transfer
    log::info!("Transferred {} tokens from {} to {}", amount, from, to);
    
    Ok(transaction)
}

pub fn get_balance(address: &str) -> Result<u64, BlockchainError> {
    let max_retries = 3;
    let mut attempts = 0;

    // Normalize address format to ensure consistent lookup
    let normalized_address = if !address.starts_with("0x") {
        format!("0x{}", address)
    } else {
        address.to_string()
    };

    // Debug log the address we're checking
    log::debug!("Getting balance for normalized address: {}", normalized_address);

    while attempts < max_retries {
        match BALANCES.lock() {
            Ok(guard) => {
                // Try both with and without 0x prefix to ensure we find the balance
                let balance = guard.get(&normalized_address)
                    .or_else(|| {
                        let no_prefix = normalized_address.trim_start_matches("0x");
                        guard.get(no_prefix)
                    })
                    .unwrap_or(&0);
                
                return Ok(*balance);
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

// Modified load method to ensure balances are properly loaded
pub fn load_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;
    init_blockchain_state();
    
    // First, try to load dedicated balances if they exist
    let mut loaded_balances = HashMap::new();
    if let Ok(Some(balances_data)) = storage.load_data(b"balances") {
        if let Ok(balances) = bincode::deserialize::<HashMap<String, u64>>(&balances_data) {
            loaded_balances = balances;
            log::info!("Loaded {} balances from dedicated storage", loaded_balances.len());
        }
    }

    // Then load blockchain data and calculate balances as a fallback
    match storage.load_data(b"blockchain")? {
        Some(value) => {
            let loaded_chain: VecDeque<Block<Blake3Algorithm>> = bincode::deserialize(&value)?;
            
            // Calculate balances and total tokens if we didn't load from dedicated storage
            let mut balances = if loaded_balances.is_empty() {
                HashMap::new()
            } else {
                loaded_balances.clone()
            };
            
            let mut total_tokens = 0;
            let mut block_height_cache = HashMap::new();
            
            // Update blockchain data
            let mut chain = BLOCKCHAIN_DATA.chain.write().unwrap();
            *chain = loaded_chain;
            
            // Process blocks for balances and caching
            if loaded_balances.is_empty() {
                for (height, block) in chain.iter().enumerate() {
                    total_tokens += block.tokens;
                    
                    // Ensure addresses are always stored with 0x prefix
                    let miner_address = normalize_address(&block.address);
                    
                    *balances.entry(miner_address).or_insert(0) += block.tokens;
                    block_height_cache.insert(block.hash.clone(), height);

                    for tx in &block.transactions {
                        // Normalize addresses for transactions too
                        let tx_sender = normalize_address(&tx.sender);
                        let tx_receiver = normalize_address(&tx.receiver);
                        
                        *balances.entry(tx_sender).or_insert(0) -= tx.amount;
                        *balances.entry(tx_receiver).or_insert(0) += tx.amount;
                    }
                }
            } else {
                // Just calculate total tokens and build cache
                for (height, block) in chain.iter().enumerate() {
                    total_tokens += block.tokens;
                    block_height_cache.insert(block.hash.clone(), height);
                }
            }

            // Update BLOCKCHAIN_DATA
            BLOCKCHAIN_DATA.total_tokens.store(total_tokens, Ordering::Relaxed);
            *BLOCKCHAIN_DATA.block_height_cache.write().unwrap() = block_height_cache;
            
            // Update balances
            *BALANCES.lock().unwrap() = balances;

            log::info!("Blockchain loaded successfully with {} blocks and {} accounts", 
                chain.len(), BALANCES.lock().unwrap().len());
        }
        None => {
            log::info!("No blockchain data found, initializing new chain");
            *BLOCKCHAIN_DATA.chain.write().unwrap() = VecDeque::new();
            BLOCKCHAIN_DATA.total_tokens.store(0, Ordering::Relaxed);
            *BALANCES.lock().unwrap() = HashMap::new();
        }
    }

    storage.flush()?;
    Ok(())
}
