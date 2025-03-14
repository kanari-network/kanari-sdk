use log::debug;

use crate::blockchain::BALANCES;

use super::{get_mutex_lock, SimulationError};

// Helper function to apply VM state changes to the blockchain
pub fn apply_state_changes(changes: &mona_vm::ChangeSet) -> Result<(), SimulationError> {
    // Lock balances for minimum time needed
    let mut balances = get_mutex_lock(&BALANCES, "apply_state_changes")?;

    // Apply writes with proper error handling
    for (key, value) in changes.get_writes() {
        if key.len() != 32 {
            debug!("Invalid key length in state change: {}, skipping", key.len());
            continue;
        }
        
        // Convert key to address
        let address = hex::encode(key);
        
        // Convert value to u64 balance
        if value.len() < 8 {
            debug!("Invalid value length in state change: {}, skipping", value.len());
            continue;
        }
        
        let balance = match value[..8].try_into() {
            Ok(bytes) => u64::from_le_bytes(bytes),
            Err(e) => {
                debug!("Failed to convert value bytes to u64: {:?}, skipping", e);
                continue;
            }
        };
        
        // Update balance
        debug!("Updating balance for address {}: {}", address, balance);
        balances.insert(address, balance);
    }

    // Apply deletes
    for key in changes.get_deletes() {
        if key.len() != 32 {
            debug!("Invalid key length in state delete: {}, skipping", key.len());
            continue;
        }
        
        let address = hex::encode(key);
        debug!("Removing balance for address: {}", address);
        balances.remove(&address);
    }

    Ok(())
}