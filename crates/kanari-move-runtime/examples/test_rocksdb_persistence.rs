use anyhow::Result;
use kanari_move_runtime::{
    state::{Account, StateManager},
    storage::persistent_store::PersistentStore,
};
use kanari_types::address::Address;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<()> {
    // Setup temporary DB path
    let db_path = PathBuf::from("./test_db_persistence");
    if db_path.exists() {
        std::fs::remove_dir_all(&db_path)?;
    }

    let addr = Address::from_hex_literal("0x1234567890abcdef1234567890abcdef12345678").unwrap();
    let acc_addr =
        Address::parse_to_account_address("0x1234567890abcdef1234567890abcdef12345678").unwrap();

    println!("1. Creating StateManager with RocksDB at {:?}", db_path);
    {
        let store = Arc::new(PersistentStore::open_with_path(Some(db_path.clone())).unwrap());
        let mut state = StateManager::new(store);

        // Create an account
        println!("2. Creating account {:?} with balance 1000", addr);
        let mut account = Account::new(acc_addr, 1000);
        account.increment_sequence();
        state.save_account(&account)?;

        // Commit changes
        println!("3. Committing state to disk...");
        state.commit()?;

        // Compute state root
        let root = state.compute_state_root();
        println!("   State root: {}", hex::encode(root));
    }

    println!("4. Re-opening StateManager from disk...");
    {
        let store = Arc::new(PersistentStore::open_with_path(Some(db_path.clone())).unwrap());
        let state = StateManager::new(store);

        // Verify account
        println!("5. Verifying account state...");
        let account = state.get_account(&acc_addr).expect("Account should exist");

        assert_eq!(account.balance, 1000, "Balance should be 1000");
        assert_eq!(account.sequence_number, 1, "Sequence number should be 1");

        println!("   Account verification successful!");
        println!("   Balance: {}", account.balance);
        println!("   Sequence: {}", account.sequence_number);

        // Verify state root matches
        let root = state.compute_state_root();
        println!("   State root: {}", hex::encode(root));
    }

    // Cleanup
    std::fs::remove_dir_all(&db_path)?;
    println!("6. Test completed successfully!");

    Ok(())
}
