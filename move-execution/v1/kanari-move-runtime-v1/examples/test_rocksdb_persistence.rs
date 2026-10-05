#![allow(clippy::print_stdout)]

use anyhow::Result;
use kanari_move_runtime_v1::{
    ChangeSet, state::StateManager, storage::persistent_store::PersistentStore,
};
use kanari_types::address::Address;
use kanari_types::gas_coin::GAS_COIN;
use std::sync::Arc;

fn main() -> Result<()> {
    // Scratch DB in the system temp directory. A relative path would drop a
    // few hundred KB of RocksDB files into the crate directory, and an
    // aborted run (the invariant checks below panic) would leave them behind
    // as untracked files in the repo.
    let db_path = std::env::temp_dir().join(format!(
        "kanari-move-runtime-rocksdb-example-{}",
        std::process::id()
    ));
    if db_path.exists() {
        std::fs::remove_dir_all(&db_path)?;
    }

    let addr = Address::from_hex_literal("0x1234567890abcdef1234567890abcdef12345678").unwrap();
    let acc_addr =
        Address::parse_to_account_address("0x1234567890abcdef1234567890abcdef12345678").unwrap();

    println!("1. Creating StateManager with RocksDB at {:?}", db_path);
    let store1 = Arc::new(PersistentStore::open_with_path(Some(db_path.clone())).unwrap());
    let mut state1 = StateManager::new(store1.clone());

    // Mint 1000 MIST through the canonical supply path. Writing the ledger
    // directly would create treasury supply that was never issued, which the
    // fail-closed startup invariant check rejects on re-open.
    println!("2. Minting 1000 MIST to owner {:?}", addr);
    let mut changeset = ChangeSet::new();
    changeset.mint(acc_addr, 1000);
    state1.apply_changeset(&changeset)?;

    // Commit changes (the mint bumps total_supply and the owner ledger together)
    println!("3. Committing state to disk...");
    state1.commit()?;

    // Compute state root
    let root = state1.compute_state_root();
    println!("   State root: {}", hex::encode(root));

    // Explicitly drop to release file handles
    drop(state1);
    drop(store1);

    // Force RocksDB to release files
    std::thread::sleep(std::time::Duration::from_millis(1000));

    println!("4. Re-opening StateManager from disk...");
    let store2 = Arc::new(PersistentStore::open_with_path(Some(db_path.clone())).unwrap());
    let state2 = StateManager::new(store2.clone());

    // Verify owner state and that supply invariants survive the restart
    println!("5. Verifying owner state...");
    assert_eq!(
        state2.total_supply,
        11_000_000_000_000_000u64 + 1000,
        "total supply should be the genesis supply plus the minted 1000 MIST"
    );
    let owner_state = state2
        .get_owner_state(&acc_addr)
        .expect("Owner state should exist");

    assert_eq!(
        owner_state.get_token_balance(GAS_COIN),
        1000,
        "Balance should be 1000"
    );
    assert_eq!(
        owner_state.nonce, 0,
        "Owner sequence is a legacy field and should remain 0"
    );

    println!("   Owner-state verification successful!");
    println!("   Balance: {}", owner_state.get_token_balance(GAS_COIN));
    println!("   Nonce: {}", owner_state.nonce);

    // Verify state root matches
    let root = state2.compute_state_root();
    println!("   State root: {}", hex::encode(root));

    // Explicitly drop before cleanup
    drop(state2);
    drop(store2);

    // Give OS time to release all file handles
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Force garbage collection
    #[cfg(target_os = "windows")]
    {
        // On Windows, RocksDB may hold file locks longer
        // Try multiple times with delays
        for attempt in 1..=5 {
            match std::fs::remove_dir_all(&db_path) {
                Ok(_) => {
                    println!("6. Cleanup successful on attempt {}!", attempt);
                    println!("7. Test completed successfully!");
                    return Ok(());
                }
                Err(_) if attempt < 5 => {
                    std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
                }
                Err(e) => {
                    eprintln!(
                        "6. Warning: Could not clean up test database after 5 attempts: {}",
                        e
                    );
                    eprintln!("   You can manually delete: {:?}", db_path);
                    println!("7. Test completed successfully!");
                    return Ok(());
                }
            }
        }
    }

    // Cleanup for non-Windows/manual path handling
    println!("6. Cleaning up test database...");
    match std::fs::remove_dir_all(&db_path) {
        Ok(_) => println!("   Cleanup successful!"),
        Err(e) => {
            eprintln!("   Warning: Could not clean up test database: {}", e);
            eprintln!("   You can manually delete: {:?}", db_path);
        }
    }
    println!("7. Test completed successfully!");

    Ok(())
}
