#![no_main]

use centauri::consensus::{DagConsensus, DagVertex};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: Test consensus with random sync-like scenarios

    if data.len() < 20 {
        return;
    }

    // Create a simple consensus instance
    let authorities = vec![
        "validator_1".to_string(),
        "validator_2".to_string(),
        "validator_3".to_string(),
        "validator_4".to_string(),
    ];

    let mut consensus = DagConsensus::with_chain_id(
        authorities[0].clone(),
        authorities.clone(),
        "sync-fuzz".to_string(),
    );

    // Add some vertices to simulate sync scenario
    let num_vertices = std::cmp::min((data[0] % 50) as usize, 50);

    for i in 0..num_vertices {
        if data.len() < 1 + i * 40 {
            break;
        }

        let round = (i / 4) as u64;
        let author_idx = i % 4;
        let author = authorities[author_idx].clone();

        // Generate state root from fuzzed data
        let mut state_root = [0u8; 32];
        let offset = 1 + i * 40;
        if offset + 32 <= data.len() {
            state_root.copy_from_slice(&data[offset..offset + 32]);
        }

        let vertex = DagVertex::new(
            round,
            author,
            "sync-fuzz".to_string(),
            vec![], // No parents for simplicity
            vec![],
            state_root.to_vec(),
            1_000_000_000u64 + round,
        );

        // Should handle any valid configuration without panicking
        let _ = consensus.add_vertex(vertex);
    }

    // Try commit - should not panic
    let _ = consensus.try_commit();

    // Verify system remains stable
    assert!(consensus.store().num_authorities() > 0);
});
