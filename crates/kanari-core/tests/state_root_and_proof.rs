use kanari_core::BlockchainEngine;

#[test]
fn state_root_and_proof_api_smoke() {
    // Create engine (in-memory by default)
    let engine = BlockchainEngine::new().expect("engine init");

    // Latest state root should be available (genesis or computed)
    let root = engine.get_state_root(None).expect("state root available");
    assert!(!root.is_empty());

    // Block data for latest height should include same state_root
    let stats = engine.get_stats();
    let block = engine.get_block(stats.height).expect("block exists");
    assert_eq!(block.state_root, root);

    // Requesting an account proof must not panic; may return None if SMT not configured
    let _ = engine.get_account_proof("0x1");
}
