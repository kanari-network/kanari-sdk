#![no_main]

use centauri::consensus::DagConsensus;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: Test multi-round consensus with random validator behavior

    if data.len() < 20 {
        return;
    }

    // Extract configuration from fuzzed data
    let num_validators = std::cmp::max((data[0] % 10) as usize, 4); // Min 4 for BFT
    let num_rounds = std::cmp::min((data[1] % 20) as usize, 20); // Max 20 rounds
    let vertices_per_round = std::cmp::max((data[2] % 5) as usize, 1);

    // Create authorities
    let authorities: Vec<String> = (0..num_validators)
        .map(|i| format!("validator_{}", i))
        .collect();

    // Initialize consensus
    let mut consensus = DagConsensus::with_chain_id(
        authorities[0].clone(),
        authorities.clone(),
        "fuzz-test-chain".to_string(),
    );

    let mut previous_vertex_ids = Vec::new();

    // Simulate multiple rounds
    for round in 0..num_rounds {
        let mut current_vertex_ids = Vec::new();

        for v in 0..vertices_per_round {
            // Select author for this vertex
            let author_idx = (round + v) % num_validators;
            let author = authorities[author_idx].clone();

            // Select parents from previous round (or empty for round 0)
            let parents = if round == 0 || previous_vertex_ids.is_empty() {
                vec![]
            } else {
                // Pick up to 3 random parents from previous round
                let num_parents = std::cmp::min(previous_vertex_ids.len(), 3);
                previous_vertex_ids
                    .iter()
                    .take(num_parents)
                    .cloned()
                    .collect()
            };

            // Generate state root from fuzzed data
            let mut state_root = [0u8; 32];
            let offset = 3 + (round * vertices_per_round + v) * 32;
            if offset + 32 <= data.len() {
                state_root.copy_from_slice(&data[offset..offset + 32]);
            }

            // Create and add vertex
            let vertex = centauri::consensus::DagVertex::new(
                round as u64,
                author,
                "fuzz-test-chain".to_string(),
                parents,
                vec![],
                state_root.to_vec(),
                1_000_000_000u64 + round as u64,
            );

            // Should handle any valid configuration without panicking
            let _ = consensus.add_vertex(vertex.clone());
            current_vertex_ids.push(vertex.id);
        }

        previous_vertex_ids = current_vertex_ids;

        // Try to commit at each round (may or may not succeed)
        let _ = consensus.try_commit();
    }

    // Verify system remains stable after multiple rounds
    assert!(consensus.store().num_authorities() > 0);

    // Check that committee is still valid
    let committee = consensus.committee();
    assert!(committee.validators.len() >= 4);
});
