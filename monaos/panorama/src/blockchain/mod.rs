use crate::block::Block;
use bincode;
use consensus_pos::Blake3Algorithm;
// Replace with common import
use common::get_kari_dir;
use mona_storage::{BlockchainStorage, RocksDBStorage, StorageError};
use mona_types::address::Address;
use crate::block::Transaction;
use std::time::{SystemTime, UNIX_EPOCH};

// Import key directly for signing functions
use key::{sign_message, verify_signature};

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

pub fn save_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;

    // Use the new BLOCKCHAIN_DATA structure
    let data = bincode::serialize(&BLOCKCHAIN_DATA.chain.read().unwrap().clone())?;
    storage.save_data(b"blockchain", &data)?;
    storage.flush()?;

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

pub fn load_blockchain() -> Result<(), StorageError> {
    let kari_dir = get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = RocksDBStorage::new(db_path)?;
    init_blockchain_state();

    match storage.load_data(b"blockchain")? {
        Some(value) => {
            let loaded_chain: VecDeque<Block<Blake3Algorithm>> = bincode::deserialize(&value)?;
            
            // Calculate balances and total tokens
            let mut balances = HashMap::new();
            let mut total_tokens = 0;
            let mut block_height_cache = HashMap::new();
            
            // Update blockchain data
            let mut chain = BLOCKCHAIN_DATA.chain.write().unwrap();
            *chain = loaded_chain;
            
            // Process blocks for balances and caching
            for (height, block) in chain.iter().enumerate() {
                total_tokens += block.tokens;
                // Ensure addresses are always stored with 0x prefix
                let miner_address = if !block.address.starts_with("0x") {
                    format!("0x{}", block.address)
                } else {
                    block.address.clone()
                };
                
                *balances.entry(miner_address).or_insert(0) += block.tokens;
                block_height_cache.insert(block.hash.clone(), height);

                for tx in &block.transactions {
                    // Normalize addresses for transactions too
                    let tx_sender = if !tx.sender.starts_with("0x") {
                        format!("0x{}", tx.sender)
                    } else {
                        tx.sender.clone()
                    };
                    
                    let tx_receiver = if !tx.receiver.starts_with("0x") {
                        format!("0x{}", tx.receiver)
                    } else {
                        tx.receiver.clone()
                    };
                    
                    *balances.entry(tx_sender).or_insert(0) -= tx.amount;
                    *balances.entry(tx_receiver).or_insert(0) += tx.amount;
                }
            }

            // Update BLOCKCHAIN_DATA
            BLOCKCHAIN_DATA.total_tokens.store(total_tokens, Ordering::Relaxed);
            *BLOCKCHAIN_DATA.block_height_cache.write().unwrap() = block_height_cache;
            
            // Update balances
            *BALANCES.lock().unwrap() = balances;

            println!("Blockchain loaded successfully");
        }
        None => {
            println!("No blockchain data found, initializing new chain");
            *BLOCKCHAIN_DATA.chain.write().unwrap() = VecDeque::new();
            BLOCKCHAIN_DATA.total_tokens.store(0, Ordering::Relaxed);
            *BALANCES.lock().unwrap() = HashMap::new();
        }
    }

    storage.flush()?;
    Ok(())
}

// Add this new function to transfer tokens between addresses
pub fn transfer_tokens(
    sender_address: &str, 
    receiver_address: &str, 
    amount: u64,
    private_key: &str
) -> Result<String, BlockchainError> {
    // Remove any 0x prefix from private key if present
    let private_key = private_key.trim_start_matches("0x");
    
    // Check if sender has enough balance
    let sender_balance = get_balance(sender_address)?;
    if sender_balance < amount {
        return Err(BlockchainError::Balance(
            format!("Insufficient balance: {} has only {} tokens", sender_address, sender_balance)
        ));
    }
    
    // Create transaction
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    
    // Normalize sender and receiver addresses consistently
    let normalized_sender = if !sender_address.starts_with("0x") {
        format!("0x{}", sender_address)
    } else {
        sender_address.to_string()
    };
    
    let normalized_receiver = if !receiver_address.starts_with("0x") {
        format!("0x{}", receiver_address)
    } else {
        receiver_address.to_string()
    };
    
    // Create the message to sign using normalized addresses
    let message = format!("{}{}{}{}", normalized_sender, normalized_receiver, amount, timestamp);
    
    log::info!("Creating transaction message: {}", message);
    log::info!("From: {}, To: {}, Amount: {}, Timestamp: {}", 
               normalized_sender, normalized_receiver, amount, timestamp);
    
    // Sign the transaction with the private key
    let signature = match sign_message(&message, private_key) {
        Ok(sig) => {
            log::info!("Signature created successfully: {}", sig);
            sig
        },
        Err(e) => return Err(BlockchainError::Balance(format!("Signature error: {}", e)))
    };
    
    // Create transaction with normalized addresses
    let transaction = Transaction {
        sender: normalized_sender.clone(), // Use normalized address
        receiver: normalized_receiver.clone(), // Use normalized address
        amount,
        timestamp,
        signature: Some(signature.clone()),
    };
    
    // Verify that this is a valid transaction
    if !verify_transaction(&transaction) {
        // Add detailed logging here
        log::error!("Transaction verification failed");
        log::error!("Signature: {}", signature);
        log::error!("Address: {}", normalized_sender);
        return Err(BlockchainError::Balance("Transaction signature verification failed".into()));
    }
    
    // Create a new block with this transaction
    let transactions = vec![transaction];
    let prev_block = if BLOCKCHAIN_DATA.len() > 0 {
        BLOCKCHAIN_DATA.get_block(BLOCKCHAIN_DATA.len() - 1)
            .ok_or_else(|| BlockchainError::Initialization("Failed to get last block".into()))?
    } else {
        return Err(BlockchainError::Initialization("Blockchain is empty".into()));
    };
    
    // Include transaction data in block
    let data = format!("Transfer: {} -> {} ({} tokens)", 
        sender_address, receiver_address, amount).into_bytes();
    
    // Create the new block
    use consensus_pos::Blake3Algorithm;
    let new_block = Block::new(
        prev_block.index + 1,
        data,
        prev_block.hash.clone(),
        0, // No new tokens minted in transfer
        transactions.clone(),
        String::new(), // No specific miner address for transfers
        Blake3Algorithm::new(),
    );
    
    // Add block to blockchain
    BLOCKCHAIN_DATA.add_block(new_block.clone());
    
    // Update balances using normalized addresses
    {
        let mut balances = BALANCES.lock().unwrap();
        *balances.entry(normalized_sender).or_insert(0) -= amount;
        *balances.entry(normalized_receiver).or_insert(0) += amount;
    }
    
    // Save the blockchain
    save_blockchain()?;
    
    // Return transaction ID (using block hash as reference)
    Ok(format!("Transaction successful! Block hash: {}", new_block.hash))
}

// Function to verify a transaction's signature
fn verify_transaction(transaction: &Transaction) -> bool {
    // If no signature, reject
    let signature = match &transaction.signature {
        Some(sig) => sig,
        None => {
            log::warn!("Transaction rejected: Missing signature");
            return false;
        }
    };
    
    // Recreate the message that was signed
    // Note: transaction.sender should already be normalized
    let message = format!("{}{}{}{}", 
        transaction.sender, 
        transaction.receiver, 
        transaction.amount, 
        transaction.timestamp
    );
    
    log::info!("Verifying transaction with message: {}", message);
    log::info!("From: {}, To: {}, Amount: {}, Timestamp: {}", 
              transaction.sender, transaction.receiver, transaction.amount, transaction.timestamp);
    
    // Verify the signature against the sender's public key (which is their address)
    match verify_signature(&message, signature, &transaction.sender) {
        Ok(valid) => {
            if !valid {
                log::warn!("Transaction signature verification failed");
            } else {
                log::info!("Transaction signature verified successfully");
            }
            valid
        },
        Err(e) => {
            log::error!("Signature verification error: {:?}", e);
            false
        }
    }
}

// Add a debugging function to directly verify signatures for troubleshooting
pub fn debug_verify_signature(
    message: &str,
    signature: &str,
    address: &str
) -> bool {
    match verify_signature(message, signature, address) {
        Ok(valid) => valid,
        Err(e) => {
            log::error!("Debug signature verification error: {:?}", e);
            false
        }
    }
}