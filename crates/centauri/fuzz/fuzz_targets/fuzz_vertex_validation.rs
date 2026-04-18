#![no_main]

use centauri::consensus::DagVertex;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz target: Test DagVertex creation and validation with random inputs

    if data.len() < 64 {
        return; // Need minimum data for meaningful test
    }

    // Extract components from fuzzed data
    let round = u64::from_le_bytes(data[0..8].try_into().unwrap_or([0u8; 8]));
    let author_len = std::cmp::min(data[8] as usize, 50); // Limit author length
    let author = String::from_utf8_lossy(&data[9..9 + author_len]).to_string();

    if author.is_empty() {
        return;
    }

    // Generate parent IDs (up to 5 parents)
    let num_parents = std::cmp::min((data[9 + author_len] % 6) as usize, 5);
    let mut parents = Vec::new();
    let mut offset = 10 + author_len;

    for _ in 0..num_parents {
        if offset + 32 <= data.len() {
            let mut parent_id = [0u8; 32];
            parent_id.copy_from_slice(&data[offset..offset + 32]);
            parents.push(parent_id);
            offset += 32;
        } else {
            break;
        }
    }

    // Generate state root (32 bytes)
    let mut state_root = [0u8; 32];
    if offset + 32 <= data.len() {
        state_root.copy_from_slice(&data[offset..offset + 32]);
    }

    // Generate timestamp
    let timestamp = if offset + 32 + 8 <= data.len() {
        u64::from_le_bytes(
            data[offset + 32..offset + 40]
                .try_into()
                .unwrap_or([0u8; 8]),
        )
    } else {
        1_000_000_000u64
    };

    // Create vertex - this should never panic regardless of input
    let _vertex = DagVertex::new(
        round,
        author.clone(),
        "fuzz-chain".to_string(),
        parents.clone(),
        vec![], // Empty transactions for fuzzing
        state_root.to_vec(),
        timestamp,
    );

    // Note: Direct field access removed due to privacy/encapsulation.
    // The primary goal is to ensure construction does not panic.
});
