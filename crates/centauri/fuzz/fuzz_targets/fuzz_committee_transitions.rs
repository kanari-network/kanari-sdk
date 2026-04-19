#![no_main]

use centauri::consensus::DagConsensus;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: Test committee changes through consensus API

    if data.len() < 10 {
        return;
    }

    // Extract configuration
    let num_validators = std::cmp::max((data[0] % 10) as usize, 4); // Min 4 for BFT

    // Create authorities
    let authorities: Vec<String> = (0..num_validators)
        .map(|i| format!("validator_{}", i))
        .collect();

    // Initialize consensus with different configurations
    let mut consensus = DagConsensus::with_chain_id(
        authorities[0].clone(),
        authorities.clone(),
        "committee-fuzz".to_string(),
    );

    // Verify initial state
    let committee = consensus.committee();
    assert_eq!(committee.epoch, 0);
    assert!(committee.validators.len() >= 4);

    // Simulate adding vertices from different validators
    let num_vertices = std::cmp::min((data[1] % 20) as usize, 20);

    for i in 0..num_vertices {
        let author_idx = i % num_validators;
        let author = authorities[author_idx].clone();

        let mut state_root = [0u8; 32];
        let offset = 2 + i * 32;
        if offset + 32 <= data.len() {
            state_root.copy_from_slice(&data[offset..offset + 32]);
        }

        let vertex = centauri::consensus::DagVertex::new(
            i as u64,
            author,
            "committee-fuzz".to_string(),
            vec![],
            vec![],
            state_root.to_vec(),
            1_000_000_000u64 + i as u64,
        );

        let _ = consensus.add_vertex(vertex);
    }

    // Try to commit
    let _ = consensus.try_commit();

    // Verify committee is still valid after operations
    let final_committee = consensus.committee();
    assert!(final_committee.validators.len() >= 4);

    // Verify quorum calculation
    let f = (final_committee.validators.len() - 1) / 3;
    let quorum = 2 * f + 1;
    assert!(quorum <= final_committee.validators.len());
});
