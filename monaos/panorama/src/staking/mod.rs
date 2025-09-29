use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock, atomic::AtomicU64};
use lazy_static::lazy_static;
use mona_storage::BlockchainStorage;
use mona_types::address::Address;
use mona_types::kari::{
    KA_PER_KARI, NODE_STAKING_MINIMUM_KA, VALIDATOR_STAKING_MINIMUM_KA,
    STAKING_REWARD_PERCENTAGE, POOL_ADDRESS, POOL_RESERVED_KA
};
use log::{debug, info, warn, error};
use serde::{Serialize, Deserialize};
use bincode;
use std::time::{SystemTime, UNIX_EPOCH};
use mona_blockchain::blockchain::{BlockchainError, BALANCES, normalize_address};
use mona_crypto::hash_data_blake3;

// Constants for staking
pub const STAKING_LOCK_PERIOD_SECONDS: u64 = 86400; // 24 hours lock period
pub const REWARDS_PER_BLOCK: u64 = 100_000; // Base rewards per block in KA

// Staking data structures
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakedNode {
    pub address: Address,
    pub staked_amount: u64,
    pub is_validator: bool,
    pub staked_at: u64,
    pub unlock_time: u64,
    pub last_reward_time: u64,
    pub accumulated_rewards: u64,
    pub security_hash: String, // Add security hash
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakingPool {
    pub total_staked: u64,
    pub nodes_count: usize,
    pub validators_count: usize,
    pub total_rewards_distributed: u64, // Track total rewards from pool
}

// Thread-safe staking globals
lazy_static! {
    pub static ref STAKING_NODES: RwLock<HashMap<String, StakedNode>> = RwLock::new(HashMap::new());
    pub static ref NODE_HASH_CACHE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
    pub static ref STAKING_POOL: Mutex<StakingPool> = Mutex::new(StakingPool {
        total_staked: 0,
        nodes_count: 0,
        validators_count: 0,
        total_rewards_distributed: 0,
    });
    pub static ref ACTIVE_VALIDATORS: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
    pub static ref POOL_REMAINING_REWARDS: AtomicU64 = AtomicU64::new(POOL_RESERVED_KA);
}

// Main staking functions
pub fn stake_tokens(
    address: &Address,
    amount: u64,
    wants_to_validate: bool
) -> Result<StakedNode, BlockchainError> {
    // Check minimum staking requirements
    if amount < NODE_STAKING_MINIMUM_KA {
        return Err(BlockchainError::Transaction(
            format!("Staking amount {} is below minimum required ({})",
                amount, NODE_STAKING_MINIMUM_KA)
        ));
    }

    let address_str = address.to_hex_literal();
    
    // Verify user has enough balance
    let balance = mona_blockchain::blockchain::get_balance(&address_str)?;
    if balance < amount {
        return Err(BlockchainError::InsufficientFunds(
            format!("Address {} has insufficient balance for staking", address_str)
        ));
    }
    
    // Calculate lock period
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let unlock_time = current_time + STAKING_LOCK_PERIOD_SECONDS;
    
    // Determine if can be validator
    let is_validator = wants_to_validate && amount >= VALIDATOR_STAKING_MINIMUM_KA;
    
    // Calculate node identity hash for verification
    let node_identity = format!("{}:{}:{}", address_str, amount, current_time);
    let node_hash = hex::encode(hash_data_blake3(node_identity.as_bytes()));
    
    // Create staked node with security hash
    let staked_node = StakedNode {
        address: *address,
        staked_amount: amount,
        is_validator,
        staked_at: current_time,
        unlock_time,
        last_reward_time: current_time,
        accumulated_rewards: 0,
        security_hash: node_hash.clone(),
    };
    
    // Update staking pool
    {
        // Lock balances and deduct staked amount
        let mut balances = BALANCES.lock().map_err(|_| 
            BlockchainError::Transaction("Failed to lock balances".to_string())
        )?;
        if let Some(user_balance) = balances.get_mut(&address_str) {
            if *user_balance < amount {
                return Err(BlockchainError::InsufficientFunds(
                    "Insufficient balance during staking lock".to_string()
                ));
            }
            *user_balance -= amount;
        } else {
            return Err(BlockchainError::InsufficientFunds(
                "User balance not found during staking process".to_string()
            ));
        }
    }
    
    // Update staking data with security hash
    {
        let mut staking_nodes = STAKING_NODES.write().map_err(|_| 
            BlockchainError::Transaction("Failed to lock staking nodes".to_string())
        )?;
        let mut staking_pool = STAKING_POOL.lock().map_err(|_| 
            BlockchainError::Transaction("Failed to lock staking pool".to_string())
        )?;
        let mut active_validators = ACTIVE_VALIDATORS.write().map_err(|_| 
            BlockchainError::Transaction("Failed to lock active validators".to_string())
        )?;
        let mut hash_cache = NODE_HASH_CACHE.write().map_err(|_| 
            BlockchainError::Transaction("Failed to lock node hash cache".to_string())
        )?;
        
        // Update staking stats
        staking_pool.total_staked += amount;
        staking_pool.nodes_count += 1;
        
        if is_validator {
            staking_pool.validators_count += 1;
            active_validators.insert(address_str.clone());
        }
        
        // Add the staked node and security hash
        staking_nodes.insert(address_str.clone(), staked_node.clone());
        hash_cache.insert(address_str.clone(), node_hash);
    }
    
    info!(
        "Address {} staked {} KARI ({} KA). Validator status: {}",
        address_str, 
        amount as f64 / KA_PER_KARI as f64, 
        amount, 
        is_validator
    );
    
    // Save staking state
    save_staking_state()?;
    
    Ok(staked_node)
}

// Unstake tokens (with penalty if within lock period)
pub fn unstake_tokens(
    address: &Address
) -> Result<(u64, u64), BlockchainError> {
    let address_str = address.to_hex_literal();
    
    // Get staked node info
    let (staked_amount, is_validator, unlock_time, accumulated_rewards) = {
        let staking_nodes = STAKING_NODES.read().map_err(|_| 
            BlockchainError::Transaction("Failed to lock staking nodes for reading".to_string())
        )?;
        
        match staking_nodes.get(&address_str) {
            Some(node) => (
                node.staked_amount,
                node.is_validator,
                node.unlock_time,
                node.accumulated_rewards
            ),
            None => return Err(BlockchainError::Transaction(
                format!("Address {} is not staking", address_str)
            )),
        }
    };
    
    // Calculate current time and check if within lock period
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Calculate withdrawal amount (with penalty if unlocking early)
    let mut withdrawal_amount = staked_amount;
    let mut early_unlock_penalty = 0;
    
    if current_time < unlock_time {
        // Calculate penalty (10% of staked amount)
        early_unlock_penalty = staked_amount.checked_div(10)
            .ok_or_else(|| BlockchainError::Transaction("Division error in penalty calculation".to_string()))?;
        withdrawal_amount = withdrawal_amount.checked_sub(early_unlock_penalty)
            .ok_or_else(|| BlockchainError::Transaction("Underflow in withdrawal calculation".to_string()))?;
        
        warn!(
            "Early unstaking by {}. Penalty: {} KARI ({} KA)",
            address_str, 
            early_unlock_penalty as f64 / KA_PER_KARI as f64,
            early_unlock_penalty
        );
        
        // Return penalty to the reward pool
        if early_unlock_penalty > 0 {
            let pool_address = match normalize_address(POOL_ADDRESS) {
                Ok(addr) => addr.to_hex_literal(),
                Err(_) => {
                    error!("Invalid pool address, penalty lost: {} KA", early_unlock_penalty);
                    // Continue without adding penalty to pool
                    String::new()
                }
            };
            
            if !pool_address.is_empty() {
                let mut balances = BALANCES.lock().map_err(|_| 
                    BlockchainError::Transaction("Failed to lock balances for penalty".to_string())
                )?;
                let pool_balance = balances.entry(pool_address).or_insert(0);
                *pool_balance = pool_balance.checked_add(early_unlock_penalty)
                    .ok_or_else(|| BlockchainError::Transaction("Overflow in pool penalty addition".to_string()))?;
            }
        }
    }
    
    // Update staking data
    {
        let mut staking_nodes = STAKING_NODES.write().map_err(|_| 
            BlockchainError::Transaction("Failed to lock staking nodes for update".to_string())
        )?;
        let mut staking_pool = STAKING_POOL.lock().map_err(|_| 
            BlockchainError::Transaction("Failed to lock staking pool for update".to_string())
        )?;
        let mut active_validators = ACTIVE_VALIDATORS.write().map_err(|_| 
            BlockchainError::Transaction("Failed to lock active validators for update".to_string())
        )?;
        
        staking_pool.total_staked -= staked_amount;
        staking_pool.nodes_count -= 1;
        
        if is_validator {
            staking_pool.validators_count -= 1;
            active_validators.remove(&address_str);
        }
        
        // Remove the staked node
        staking_nodes.remove(&address_str);
    }
    
    // Return tokens to user's balance (minus penalty if applicable)
    {
        let mut balances = BALANCES.lock().map_err(|_| 
            BlockchainError::Transaction("Failed to lock balances for withdrawal".to_string())
        )?;
        let total_return = withdrawal_amount.checked_add(accumulated_rewards)
            .ok_or_else(|| BlockchainError::Transaction("Integer overflow in withdrawal calculation".to_string()))?;
        let current_balance = balances.entry(address_str.clone()).or_insert(0);
        *current_balance = current_balance.checked_add(total_return)
            .ok_or_else(|| BlockchainError::Transaction("Integer overflow in balance update".to_string()))?;
    }
    
    info!(
        "Address {} unstaked {} KARI ({} KA) with {} KARI penalty. Rewards: {} KARI",
        address_str,
        staked_amount as f64 / KA_PER_KARI as f64,
        staked_amount,
        early_unlock_penalty as f64 / KA_PER_KARI as f64,
        accumulated_rewards as f64 / KA_PER_KARI as f64
    );
    
    // Save staking state
    save_staking_state()?;
    
    Ok((withdrawal_amount, accumulated_rewards))
}

// Verify a node's integrity using its security hash
pub fn verify_node_integrity(address: &Address) -> bool {
    let address_str = address.to_hex_literal();
    
    // Get the node and its security hash
    let (node, hash) = {
        let nodes = STAKING_NODES.read().unwrap();
        let hashes = NODE_HASH_CACHE.read().unwrap();
        
        match (nodes.get(&address_str), hashes.get(&address_str)) {
            (Some(node), Some(hash)) => (node.clone(), hash.clone()),
            _ => return false,
        }
    };
    
    // Recalculate the security hash
    let node_identity = format!("{}:{}:{}", address_str, node.staked_amount, node.staked_at);
    let calculated_hash = hex::encode(hash_data_blake3(node_identity.as_bytes()));
    
    // Verify hash matches
    calculated_hash == hash
}

// Calculate and distribute staking rewards
pub fn process_rewards(block_height: u32) -> Result<u64, BlockchainError> {
    let mut total_rewards = 0;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Process rewards only on every 5th block
    if block_height % 5 != 0 {
        return Ok(0);
    }
    
    // Get current pool balance - rewards come from pool
    let pool_addr_str = match normalize_address(POOL_ADDRESS) {
        Ok(addr) => addr.to_hex_literal(),
        Err(_) => return Err(BlockchainError::Transaction("Invalid pool address".to_string())),
    };
    
    let pool_balance = match mona_blockchain::blockchain::get_balance(&pool_addr_str) {
        Ok(balance) => balance,
        Err(_) => return Err(BlockchainError::Transaction("Failed to get pool balance".to_string())),
    };
    
    // If pool has no tokens left, no rewards can be distributed
    if pool_balance == 0 {
        info!("No funds left in reward pool. Staking rewards have been exhausted.");
        return Ok(0);
    }
    
    // Get list of nodes to process and calculate rewards
    let mut nodes_to_update: Vec<(String, u64)> = Vec::new();
    let mut total_reward_calculation = 0;
    
    {
        let mut staking_nodes = STAKING_NODES.write().unwrap();
        
        for (address, node) in staking_nodes.iter_mut() {
            // Skip if not a validator
            if !node.is_validator {
                continue;
            }
            
            // Calculate time since last reward
            let time_since_last_reward = current_time - node.last_reward_time;
            
            // Skip if less than an hour has passed
            if time_since_last_reward < 3600 {
                continue;
            }
            
            // Calculate reward based on staked amount and time passed
            // Daily reward rate = annual rate / 365
            let daily_reward_rate = STAKING_REWARD_PERCENTAGE / 365.0;
            
            // Convert time to days
            let days_passed = time_since_last_reward as f64 / 86400.0;
            
            // Calculate rewards: amount * daily_rate * days
            let reward = (node.staked_amount as f64 * daily_reward_rate * days_passed).round() as u64;
            
            // Add to total calculated rewards
            total_reward_calculation += reward;
            
            // Save address and reward for later processing
            nodes_to_update.push((address.clone(), reward));
        }
    }
    
    // Check if pool has sufficient funds
    if total_reward_calculation > pool_balance {
        // Scale down rewards proportionally
        let scale_factor = pool_balance as f64 / total_reward_calculation as f64;
        let mut scaled_nodes_to_update: Vec<(String, u64)> = Vec::new();
        
        for (address, reward) in nodes_to_update {
            let scaled_reward = (reward as f64 * scale_factor).round() as u64;
            scaled_nodes_to_update.push((address, scaled_reward));
            total_rewards += scaled_reward;
        }
        
        nodes_to_update = scaled_nodes_to_update;
        
        warn!(
            "Insufficient funds in reward pool. Rewards scaled down to {}% of calculated value.",
            (scale_factor * 100.0).round()
        );
    } else {
        total_rewards = total_reward_calculation;
    }
    
    // If there are no rewards to distribute, return
    if total_rewards == 0 {
        return Ok(0);
    }
    
    // Transfer rewards from pool to validators
    {
        let mut balances = BALANCES.lock().unwrap();
        
        // Decrease pool balance
        if let Some(pool_balance) = balances.get_mut(&pool_addr_str) {
            if *pool_balance < total_rewards {
                warn!("Pool has insufficient balance for rewards. Expected: {}, Actual: {}", total_rewards, *pool_balance);
                return Err(BlockchainError::InsufficientFunds(
                    format!("Insufficient funds in reward pool")
                ));
            }
            *pool_balance -= total_rewards;
        } else {
            warn!("Pool address not found in balances");
            return Err(BlockchainError::Transaction(
                format!("Pool address not found in balances")
            ));
        }
        
        // Update validator nodes with rewards
        let mut staking_nodes = STAKING_NODES.write().unwrap();
        let mut staking_pool = STAKING_POOL.lock().unwrap();
        
        // Update total rewards distributed statistic
        staking_pool.total_rewards_distributed += total_rewards;
        
        for (address, reward) in &nodes_to_update {
            // Update node accumulated rewards
            if let Some(node) = staking_nodes.get_mut(address) {
                node.accumulated_rewards += *reward;
                node.last_reward_time = current_time;
                
                debug!(
                    "Validator {} earned {} KARI ({} KA) reward from pool",
                    address,
                    *reward as f64 / KA_PER_KARI as f64,
                    reward
                );
            }
        }
    }
    
    // Save staking state
    save_staking_state()?;
    
    Ok(total_rewards)
}

// Get a list of active validators for consensus
pub fn get_active_validators() -> Vec<Address> {
    let validators = ACTIVE_VALIDATORS.read().unwrap();
    let nodes = STAKING_NODES.read().unwrap();
    
    validators.iter()
        .filter_map(|addr| {
            // Use proper filter_map to only collect the Some values
            nodes.get(addr).map(|node| node.address.clone())
        })
        .collect()
}

// Check if an address is a validator
pub fn is_validator(address: &Address) -> bool {
    let address_str = address.to_hex_literal();
    
    match ACTIVE_VALIDATORS.read() {
        Ok(validators) => validators.contains(&address_str),
        Err(_) => false,
    }
}

// Get staking info for an address
pub fn get_staking_info(address: &Address) -> Option<StakedNode> {
    let address_str = address.to_hex_literal();
    
    match STAKING_NODES.read() {
        Ok(nodes) => nodes.get(&address_str).cloned(),
        Err(_) => None,
    }
}

// Get current staking statistics
pub fn get_staking_stats() -> StakingPool {
    match STAKING_POOL.lock() {
        Ok(pool) => pool.clone(),
        Err(_) => StakingPool {
            total_staked: 0,
            nodes_count: 0,
            validators_count: 0,
            total_rewards_distributed: 0,
        },
    }
}

// Get remaining pool balance for rewards
pub fn get_pool_remaining_balance() -> Result<u64, BlockchainError> {
    let pool_addr_str = match normalize_address(POOL_ADDRESS) {
        Ok(addr) => addr.to_hex_literal(),
        Err(_) => return Err(BlockchainError::Transaction("Invalid pool address".to_string())),
    };
    
    match mona_blockchain::blockchain::get_balance(&pool_addr_str) {
        Ok(balance) => Ok(balance),
        Err(e) => Err(e),
    }
}

// Save staking state to storage
fn save_staking_state() -> Result<(), BlockchainError> {
    // Get data
    let nodes = match STAKING_NODES.read() {
        Ok(nodes) => nodes.clone(),
        Err(_) => {
            return Err(BlockchainError::Storage("Failed to read staking nodes".to_string()));
        }
    };
    
    let pool = match STAKING_POOL.lock() {
        Ok(pool) => pool.clone(),
        Err(_) => {
            return Err(BlockchainError::Storage("Failed to read staking pool".to_string()));
        }
    };
    
    let validators = match ACTIVE_VALIDATORS.read() {
        Ok(validators) => validators.clone(),
        Err(_) => {
            return Err(BlockchainError::Storage("Failed to read active validators".to_string()));
        }
    };
    
    let hashes = match NODE_HASH_CACHE.read() {
        Ok(hashes) => hashes.clone(),
        Err(_) => {
            return Err(BlockchainError::Storage("Failed to read node hash cache".to_string()));
        }
    };
    
    // Serialize data
    let nodes_data = match bincode::serialize(&nodes) {
        Ok(data) => data,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to serialize staking nodes: {}", e)
            ));
        }
    };
    
    let pool_data = match bincode::serialize(&pool) {
        Ok(data) => data,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to serialize staking pool: {}", e)
            ));
        }
    };
    
    let validators_data = match bincode::serialize(&validators) {
        Ok(data) => data,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to serialize active validators: {}", e)
            ));
        }
    };
    
    let hashes_data = match bincode::serialize(&hashes) {
        Ok(data) => data,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to serialize node hash cache: {}", e)
            ));
        }
    };
    
    // Save data
    let kari_dir = common::get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = match mona_storage::RocksDBStorage::new(db_path) {
        Ok(storage) => storage,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to open storage: {}", e)
            ));
        }
    };
    
    if let Err(e) = storage.save_data(b"staking_nodes", &nodes_data) {
        return Err(BlockchainError::Storage(
            format!("Failed to save staking nodes: {}", e)
        ));
    }
    
    if let Err(e) = storage.save_data(b"staking_pool", &pool_data) {
        return Err(BlockchainError::Storage(
            format!("Failed to save staking pool: {}", e)
        ));
    }
    
    if let Err(e) = storage.save_data(b"active_validators", &validators_data) {
        return Err(BlockchainError::Storage(
            format!("Failed to save active validators: {}", e)
        ));
    }
    
    if let Err(e) = storage.save_data(b"node_hash_cache", &hashes_data) {
        return Err(BlockchainError::Storage(
            format!("Failed to save node hash cache: {}", e)
        ));
    }
    
    Ok(())
}

// Load staking state from storage
pub fn load_staking_state() -> Result<(), BlockchainError> {
    let kari_dir = common::get_kari_dir();
    let db_path = kari_dir.join("blockchain_db");
    let storage = match mona_storage::RocksDBStorage::new(db_path) {
        Ok(storage) => storage,
        Err(e) => {
            return Err(BlockchainError::Storage(
                format!("Failed to open storage: {}", e)
            ));
        }
    };
    
    // Load nodes
    if let Ok(Some(nodes_data)) = storage.load_data(b"staking_nodes") {
        if let Ok(nodes) = bincode::deserialize::<HashMap<String, StakedNode>>(&nodes_data) {
            *STAKING_NODES.write().unwrap() = nodes;
            debug!("Loaded {} staked nodes", STAKING_NODES.read().unwrap().len());
        }
    }
    
    // Load pool
    if let Ok(Some(pool_data)) = storage.load_data(b"staking_pool") {
        if let Ok(pool) = bincode::deserialize::<StakingPool>(&pool_data) {
            // Clone pool before moving it to avoid the borrow error
            *STAKING_POOL.lock().unwrap() = pool.clone();
            debug!("Loaded staking pool: {} total staked, {} rewards distributed", 
                   pool.total_staked, pool.total_rewards_distributed);
        }
    }
    
    // Load validators
    if let Ok(Some(validators_data)) = storage.load_data(b"active_validators") {
        if let Ok(validators) = bincode::deserialize::<HashSet<String>>(&validators_data) {
            *ACTIVE_VALIDATORS.write().unwrap() = validators;
            debug!("Loaded {} active validators", ACTIVE_VALIDATORS.read().unwrap().len());
        }
    }
    
    // Load node hash cache
    if let Ok(Some(hashes_data)) = storage.load_data(b"node_hash_cache") {
        if let Ok(hashes) = bincode::deserialize::<HashMap<String, String>>(&hashes_data) {
            *NODE_HASH_CACHE.write().unwrap() = hashes;
            debug!("Loaded {} node hashes", NODE_HASH_CACHE.read().unwrap().len());
        }
    }
    
    Ok(())
}
