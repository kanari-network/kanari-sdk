use consensus_pos::Blake3Algorithm;
use log::{error, info, warn, debug};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::block::{Block, Transaction};
use crate::blockchain::{save_blockchain, BALANCES, BLOCKCHAIN_DATA};

// Constants for Kari token
/// The amount of KA per Kari token based on the the fact that KA is
/// 10^-9 of a Kari token
const KA_PER_KARI: u64 = 1_000_000_000;

/// The total supply of Kari denominated in whole Kari tokens (100 Million)
const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of Kari denominated in KA (100 Million * 10^9)
const TOTAL_SUPPLY_KA: u64 = 100_000_000_000_000_000;

// Add coin structure
#[derive(Clone, Debug)]
pub struct Coin {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
}

impl Default for Coin {
    fn default() -> Self {
        Coin {
            name: "Kanari".to_string(),
            symbol: "KARI".to_string(),
            decimals: 9, // 9 decimals for KA units
            total_supply: TOTAL_SUPPLY_KA,
        }
    }
}

pub fn run_blockchain(
    running: Arc<Mutex<bool>>, 
    address: String,
    tx: mpsc::Sender<String>
) {
    let coin = Coin::default();

    info!("Initializing blockchain with {} coin", coin.name);
    info!(
        "Total supply: {} {} ({} {})",
        TOTAL_SUPPLY_KARI,
        coin.symbol,
        TOTAL_SUPPLY_KA,
        format!("{}A", coin.symbol)
    );

    // Ensure address is normalized for consistent storage
    let normalized_address = if !address.starts_with("0x") {
        format!("0x{}", address)
    } else {
        address.clone()
    };

    debug!("Using normalized address: {}", normalized_address);

    // Check if blockchain is already initialized
    if !BLOCKCHAIN_DATA.is_empty() {
        info!(
            "Blockchain already initialized with {} blocks",
            BLOCKCHAIN_DATA.len()
        );

        // Send initial blockchain status
        let _ = tx.try_send(format!(
            "Blockchain loaded with {} blocks",
            BLOCKCHAIN_DATA.len()
        ));

        // Check balance using both original and normalized address for troubleshooting
        match crate::blockchain::get_balance(&normalized_address) {
            Ok(balance) => {
                let balance_in_kari = balance as f64 / KA_PER_KARI as f64;
                info!(
                    "Address {} has {:.9} {} ({} {}A)",
                    normalized_address, balance_in_kari, coin.symbol, balance, coin.symbol
                );
            }
            Err(e) => warn!("Failed to get balance for address {}: {}", normalized_address, e),
        }

        // Debug: Also check with original address if they're different
        if normalized_address != address {
            match crate::blockchain::get_balance(&address) {
                Ok(balance) => {
                    debug!(
                        "Original address {} has {} {}A",
                        address, balance, coin.symbol
                    );
                }
                Err(e) => debug!("Failed to get balance for original address {}: {}", address, e),
            }
        }
    } else {
        // Create genesis block with coin info
        let genesis_block = create_genesis_block(&normalized_address, &coin);
        BLOCKCHAIN_DATA.add_block(genesis_block.clone());

        // Send genesis block info
        let _ = tx.try_send(format!(
            "Genesis Block Created\nHash: {}\nMinter: {}\nTotal Supply: {} {}",
            &genesis_block.hash, normalized_address, TOTAL_SUPPLY_KARI, coin.symbol
        ));

        // Update balances with normalized address
        {
            let mut balances = BALANCES.lock().unwrap();
            balances.insert(normalized_address.clone(), coin.total_supply);
            
            // Debug: Output all balances
            debug!("Initial balances after genesis:");
            for (addr, bal) in balances.iter() {
                debug!("  {} => {}", addr, bal);
            }
        }

        match save_blockchain() {
            Ok(_) => info!("Genesis block created successfully"),
            Err(e) => error!("Failed to save blockchain: {}", e),
        }

        info!(
            "Total supply of {} {} ({} {}A) minted to: {}",
            TOTAL_SUPPLY_KARI, coin.symbol, TOTAL_SUPPLY_KA, coin.symbol, normalized_address
        );
    }

    // Start mining blocks after genesis
    loop {
        if !*running.lock().unwrap() {
            break;
        }

        // Get the previous block - store the iterator result in a variable first
        let blockchain_data = BLOCKCHAIN_DATA.iter();
        let prev_block = match blockchain_data.last() {
            Some(block) => block,
            None => {
                error!("Cannot find previous block");
                break;
            }
        };

        // Create new block data
        let data = format!(
            "Block {} - {} Coin Transaction Block",
            prev_block.index + 1,
            coin.symbol
        )
        .into_bytes();

        // Create new block
        let new_block = Block::new(
            prev_block.index + 1,
            data,
            prev_block.hash.clone(),
            0,          // No new tokens in regular blocks
            Vec::new(), // Empty transactions for now
            normalized_address.clone(), // Use normalized address consistently
            Blake3Algorithm::new(),
        );

        // Add block to chain
        BLOCKCHAIN_DATA.add_block(new_block.clone());

        // Convert timestamp to human-readable format
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(new_block.timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

        // Send block status update
        let _ = tx.try_send(format!(
            "Block #{} mined at {}\nHash: {}\nPrev Hash: {}\nNode Address: {}\nTransactions: {}",
            new_block.index,
            datetime,
            &new_block.hash,
            &new_block.prev_hash,
            normalized_address,
            new_block.transactions.len()
        ));

        // Save blockchain state
        match save_blockchain() {
            Ok(_) => info!(
                "Block {} created successfully. Hash: {}...",
                new_block.index,
                &new_block.hash[..16]
            ),
            Err(e) => error!("Failed to save blockchain: {}", e),
        }

        // Check balances periodically to ensure they're consistent
        if new_block.index % 5 == 0 {
            match crate::blockchain::get_balance(&normalized_address) {
                Ok(balance) => {
                    debug!(
                        "Current balance for {} is {} {}A",
                        normalized_address, balance, coin.symbol
                    );
                },
                Err(e) => warn!("Failed to get balance: {}", e),
            }
            
            // Debug: Log all balances for troubleshooting
            if let Ok(balances) = BALANCES.lock() {
                debug!("Current balances in system:");
                for (addr, bal) in balances.iter() {
                    debug!("  {} => {}", addr, bal);
                }
            }
        }

        // Sleep to control block creation rate
        thread::sleep(Duration::from_secs(10));
    }
}

/// Create a genesis block containing the total supply of Kari tokens
fn create_genesis_block(address: &str, coin: &Coin) -> Block<Blake3Algorithm> {
    let _timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();

    let data = format!(
        "Genesis Block - {} Coin\nName: {}\nSymbol: {}\nTotal Supply: {} {}",
        coin.name, coin.name, coin.symbol, TOTAL_SUPPLY_KARI, coin.symbol
    )
    .into_bytes();

    Block::new(
        0,
        data,
        "0".repeat(64),
        coin.total_supply,
        Vec::new(),
        address.to_string(), // Address should already be normalized by caller
        Blake3Algorithm::new(),
    )
}

