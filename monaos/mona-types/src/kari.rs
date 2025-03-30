use std::time::{SystemTime, UNIX_EPOCH};


// Constants for Kari token
/// The amount of KA per Kari token based on the the fact that KA is
/// 10^-9 of a Kari token
pub const KA_PER_KARI: u64 = 1_000_000_000;

/// The total supply of Kari denominated in whole Kari tokens (100 Million)
pub const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of Kari denominated in KA (100 Million * 10^9)
pub const TOTAL_SUPPLY_KA: u64 = 100_000_000_000_000_000;

// Enhanced KARI structure with additional properties
#[derive(Clone, Debug)]
pub struct KARI {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub max_supply: u64,      // Maximum supply that will ever exist
    pub block_reward: u64,    // Reward per block if applicable
    pub created_at: u64,      // Timestamp when KARI was created
}

impl Default for KARI {
    fn default() -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        KARI {
            name: "Kanari".to_string(),
            symbol: "KARI".to_string(),
            decimals: 9, // 9 decimals for KA units
            total_supply: TOTAL_SUPPLY_KA,
            max_supply: TOTAL_SUPPLY_KA, // Same as total supply for fixed supply
            block_reward: 0,             // No mining rewards in this implementation
            created_at: current_time,    // Current timestamp
        }
    }
}
