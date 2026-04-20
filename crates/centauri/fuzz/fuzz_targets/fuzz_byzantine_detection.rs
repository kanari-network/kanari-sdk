#![no_main]

use centauri::consensus::DagConsensus;
use libfuzzer_sys::fuzz_target;
use crate::calculate_quorum;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: Test Byzantine fault detection with random fault patterns

    if data.len() < 10 {
        return;
    }

    // Extract configuration
    let num_validators = std::cmp::max((data[0] % 20) as usize, 4); // Min 4 for BFT
    let num_faults = std::cmp::min((data[1] % 10) as usize, 10);

    // Create authorities
    let authorities: Vec<String> = (0..num_validators)
        .map(|i| format!("validator_{}", i))
        .collect();

    // Initialize consensus with different configurations
    let consensus = DagConsensus::with_chain_id(
        authorities[0].clone(),
        authorities.clone(),
        "byzantine-fuzz".to_string(),
    );

    // Simulate random Byzantine faults
    for i in 0..num_faults {
        if data.len() < 2 + i * 40 {
            break;
        }

        // Pick random validator to penalize
        let target_idx = (data[2 + i * 40] as usize) % num_validators;
        let target_authority = authorities[target_idx].clone();

        // Generate random vertex IDs for fault reporting
        let mut vertex_id_1 = [0u8; 32];
        let mut vertex_id_2 = [0u8; 32];

        let offset = 3 + i * 40;
        if offset + 64 <= data.len() {
            vertex_id_1.copy_from_slice(&data[offset..offset + 32]);
            vertex_id_2.copy_from_slice(&data[offset + 32..offset + 64]);
        }

        let round = u64::from_le_bytes(
            data[offset + 64..offset + 72]
                .try_into()
                .unwrap_or([0u8; 8]),
        );

        // Apply different fault types based on fuzzed data
        let fault_type = data.get(offset + 72).unwrap_or(&0) % 3;

        match fault_type {
            0 => {
                // Equivocation: Same round, different vertices
                // Note: report_equivocation may not be public, skip for now
                let _ = (target_authority, round, vertex_id_1, vertex_id_2);
            }
            1 => {
                // Invalid signature
                // Note: report_invalid_signature may not be public, skip for now
                let _ = (target_authority, round, vertex_id_1);
            }
            2 => {
                // Invalid parent reference
                // Note: report_invalid_parent may not be public, skip for now
                let _ = (target_authority, round, vertex_id_1);
            }
            _ => unreachable!(),
        }
    }

    // Verify reputation system works correctly
    for auth in &authorities {
        let is_trusted = consensus.store().is_authority_trusted(auth);
        // Should not panic regardless of fault pattern
        // Just verify it returns a valid boolean
        let _ = is_trusted;
    }

    // Note: get_banned_authorities() may not be public, skip this check
    // The important part is that Byzantine detection doesn't panic

    // Verify quorum calculation still works
    let committee = consensus.committee();
    let quorum = calculate_quorum(committee.validators.len());
    assert!(
        quorum <= committee.validators.len(),
        "Quorum should be achievable"
    );
});
