#![no_main]
use libfuzzer_sys::fuzz_target;
use kanari_core::engine::BlockchainEngine;
use kanari_types::transaction::SignedTransaction;

fuzz_target!(|data: &[u8]| {
    // BCS deserialization must never panic; verification must be fallible.
    if let Ok(tx) = bcs::from_bytes::<SignedTransaction>(data) {
        let _ = tx.clone().into_verified();
        let _ = tx.transaction_hash();
        let _ = tx.verify_signature();
    }
    // Also exercise decode path used by admission.
    let _ = BlockchainEngine::new_in_memory();
});