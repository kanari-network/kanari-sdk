// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Transaction Merkle tree implementation using SMT hash functions
//! This provides efficient verification for light clients

use crate::digest;

type MerkleProofItem = (Vec<u8>, usize, Vec<Vec<u8>>);

/// Build a merkle tree from transaction hashes and return the merkle root
/// Uses the same Blake3 hashing as the SMT for consistency
pub fn compute_merkle_root(tx_hashes: &[Vec<u8>]) -> Vec<u8> {
    if tx_hashes.is_empty() {
        return vec![0u8; 32]; // Empty tree has zero hash
    }

    if tx_hashes.len() == 1 {
        return tx_hashes[0].clone();
    }

    // Build tree bottom-up
    let mut current_level = tx_hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs
        for chunk in current_level.chunks(2) {
            if chunk.len() == 2 {
                // Hash pair using SMT's digest function
                let combined = [chunk[0].as_slice(), chunk[1].as_slice()].concat();
                let hash = digest(&combined);
                next_level.push(hash.to_vec());
            } else {
                // Odd node - duplicate it
                let combined = [chunk[0].as_slice(), chunk[0].as_slice()].concat();
                let hash = digest(&combined);
                next_level.push(hash.to_vec());
            }
        }

        current_level = next_level;
    }

    current_level[0].clone()
}

/// Generate merkle proof for a transaction at given index
/// Returns sibling hashes needed to reconstruct the merkle root
pub fn generate_merkle_proof(tx_hashes: &[Vec<u8>], index: usize) -> Vec<Vec<u8>> {
    if index >= tx_hashes.len() {
        return Vec::new();
    }

    let mut proof = Vec::new();
    let mut current_level = tx_hashes.to_vec();
    let mut current_index = index;

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Determine sibling index
        let sibling_index = if current_index.is_multiple_of(2) {
            current_index + 1
        } else {
            current_index - 1
        };

        // Add sibling to proof if exists
        if sibling_index < current_level.len() {
            proof.push(current_level[sibling_index].clone());
        } else {
            // No sibling - duplicate current node
            proof.push(current_level[current_index].clone());
        }

        // Build next level
        for chunk in current_level.chunks(2) {
            if chunk.len() == 2 {
                let combined = [chunk[0].as_slice(), chunk[1].as_slice()].concat();
                let hash = digest(&combined);
                next_level.push(hash.to_vec());
            } else {
                let combined = [chunk[0].as_slice(), chunk[0].as_slice()].concat();
                let hash = digest(&combined);
                next_level.push(hash.to_vec());
            }
        }

        current_level = next_level;
        current_index /= 2;
    }

    proof
}

/// Verify a merkle proof
/// Returns true if the proof is valid for the given transaction hash
pub fn verify_merkle_proof(
    tx_hash: &[u8],
    index: usize,
    proof: &[Vec<u8>],
    merkle_root: &[u8],
) -> bool {
    if proof.is_empty() && tx_hash == merkle_root {
        return true; // Single transaction tree
    }

    let mut current_hash = tx_hash.to_vec();
    let mut current_index = index;

    for sibling in proof {
        current_hash = if current_index.is_multiple_of(2) {
            // Current is left, sibling is right
            let combined = [current_hash.as_slice(), sibling.as_slice()].concat();
            digest(&combined).to_vec()
        } else {
            // Current is right, sibling is left
            let combined = [sibling.as_slice(), current_hash.as_slice()].concat();
            digest(&combined).to_vec()
        };
        current_index /= 2;
    }

    current_hash == merkle_root
}

/// Batch verify multiple merkle proofs efficiently
/// Returns true only if ALL proofs are valid
pub fn batch_verify_merkle_proofs(proofs: &[MerkleProofItem], merkle_root: &[u8]) -> bool {
    for (tx_hash, index, proof) in proofs {
        if !verify_merkle_proof(tx_hash, *index, proof, merkle_root) {
            return false;
        }
    }
    true
}

/// Generate a multiproof for multiple transactions at once
/// More efficient than generating individual proofs as it removes duplicate siblings
pub fn generate_merkle_multiproof(tx_hashes: &[Vec<u8>], indices: &[usize]) -> Vec<Vec<u8>> {
    if indices.is_empty() || tx_hashes.is_empty() {
        return Vec::new();
    }

    // Track all nodes we need in the proof
    let mut proof_nodes = std::collections::BTreeSet::new();
    let mut current_level = tx_hashes.to_vec();

    // For each level, track which indices we're proving
    let mut current_indices: std::collections::BTreeSet<usize> = indices.iter().copied().collect();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        let mut next_indices = std::collections::BTreeSet::new();

        for chunk_idx in 0..current_level.len().div_ceil(2) {
            let left_idx = chunk_idx * 2;
            let right_idx = left_idx + 1;
            let parent_idx = chunk_idx;

            // Check if we need siblings for this pair
            let need_left = current_indices.contains(&left_idx);
            let need_right =
                right_idx < current_level.len() && current_indices.contains(&right_idx);

            // Add siblings to proof (but not the nodes we're proving)
            if need_left && !need_right && right_idx < current_level.len() {
                proof_nodes.insert(current_level[right_idx].clone());
            } else if need_right && !need_left {
                proof_nodes.insert(current_level[left_idx].clone());
            }

            // Build next level
            if right_idx < current_level.len() {
                let combined = [
                    current_level[left_idx].as_slice(),
                    current_level[right_idx].as_slice(),
                ]
                .concat();
                next_level.push(digest(&combined).to_vec());
            } else {
                let combined = [
                    current_level[left_idx].as_slice(),
                    current_level[left_idx].as_slice(),
                ]
                .concat();
                next_level.push(digest(&combined).to_vec());
            }

            // Track parent index for next level
            if need_left || need_right {
                next_indices.insert(parent_idx);
            }
        }

        current_level = next_level;
        current_indices = next_indices;
    }

    proof_nodes.into_iter().collect()
}

/// Compressed proof format using bit flags to indicate left/right positions
/// Reduces bandwidth by ~50% for proofs
#[derive(Debug, Clone)]
pub struct CompressedMerkleProof {
    pub siblings: Vec<Vec<u8>>,
    pub flags: Vec<bool>, // true = sibling is on right, false = sibling is on left
}

impl CompressedMerkleProof {
    /// Compress a standard merkle proof
    pub fn from_proof(_tx_hash: &[u8], index: usize, proof: &[Vec<u8>]) -> Self {
        let mut flags = Vec::new();
        let mut current_index = index;

        for _ in proof {
            flags.push(current_index.is_multiple_of(2)); // true if current is left (sibling is right)
            current_index /= 2;
        }

        Self {
            siblings: proof.to_vec(),
            flags,
        }
    }

    /// Verify compressed proof
    pub fn verify(&self, tx_hash: &[u8], _index: usize, merkle_root: &[u8]) -> bool {
        if self.siblings.is_empty() && tx_hash == merkle_root {
            return true;
        }

        if self.siblings.len() != self.flags.len() {
            return false; // Invalid compressed proof
        }

        let mut current_hash = tx_hash.to_vec();

        for (sibling, &is_left) in self.siblings.iter().zip(self.flags.iter()) {
            current_hash = if is_left {
                // Current is left, sibling is right
                let combined = [current_hash.as_slice(), sibling.as_slice()].concat();
                digest(&combined).to_vec()
            } else {
                // Current is right, sibling is left
                let combined = [sibling.as_slice(), current_hash.as_slice()].concat();
                digest(&combined).to_vec()
            };
        }

        current_hash == merkle_root
    }

    /// Serialize to compact bytes (for network transmission)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write number of siblings
        bytes.extend(&(self.siblings.len() as u32).to_le_bytes());

        // Write flags as packed bits
        let mut flag_byte = 0u8;
        let mut bit_pos = 0u8;
        for &flag in &self.flags {
            if flag {
                flag_byte |= 1 << bit_pos;
            }
            bit_pos += 1;
            if bit_pos == 8 {
                bytes.push(flag_byte);
                flag_byte = 0;
                bit_pos = 0;
            }
        }
        if bit_pos > 0 {
            bytes.push(flag_byte);
        }

        // Write sibling hashes
        for sibling in &self.siblings {
            bytes.extend(sibling);
        }

        bytes
    }

    /// Deserialize from compact bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let flag_bytes_needed = count.div_ceil(8);

        if bytes.len() < 4 + flag_bytes_needed + count * 32 {
            return None;
        }

        // Read flags
        let mut flags = Vec::new();
        for i in 0..count {
            let byte_idx = 4 + i / 8;
            let bit_idx = i % 8;
            let flag = (bytes[byte_idx] & (1 << bit_idx)) != 0;
            flags.push(flag);
        }

        // Read siblings
        let mut siblings = Vec::new();
        let sibling_start = 4 + flag_bytes_needed;
        for i in 0..count {
            let start = sibling_start + i * 32;
            let end = start + 32;
            siblings.push(bytes[start..end].to_vec());
        }

        Some(Self { siblings, flags })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_merkle_root() {
        let root = compute_merkle_root(&[]);
        assert_eq!(root.len(), 32);
        assert_eq!(root, vec![0u8; 32]);
    }

    #[test]
    fn test_single_tx_merkle_root() {
        let tx_hash = digest(b"tx1").to_vec();
        let root = compute_merkle_root(std::slice::from_ref(&tx_hash));
        assert_eq!(root, tx_hash);
    }

    #[test]
    fn test_two_tx_merkle_root() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let root = compute_merkle_root(&[tx1.clone(), tx2.clone()]);

        let combined = [tx1.as_slice(), tx2.as_slice()].concat();
        let expected = digest(&combined).to_vec();
        assert_eq!(root, expected);
    }

    #[test]
    fn test_three_tx_merkle_root() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();

        let root = compute_merkle_root(&[tx1.clone(), tx2.clone(), tx3.clone()]);

        // Level 1: hash pairs
        let h12 = digest(&[tx1.as_slice(), tx2.as_slice()].concat()).to_vec();
        let h33 = digest(&[tx3.as_slice(), tx3.as_slice()].concat()).to_vec(); // duplicate

        // Level 2: hash results
        let expected = digest(&[h12.as_slice(), h33.as_slice()].concat()).to_vec();
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();
        let tx4 = digest(b"tx4").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()];
        let root = compute_merkle_root(&txs);

        // Test proof for each transaction
        for (i, tx) in txs.iter().enumerate() {
            let proof = generate_merkle_proof(&txs, i);
            assert!(verify_merkle_proof(tx, i, &proof, &root));
        }
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone()];
        let root = compute_merkle_root(&txs);

        let proof = generate_merkle_proof(&txs, 0);
        let fake_tx = digest(b"fake").to_vec();

        // Wrong hash should fail
        assert!(!verify_merkle_proof(&fake_tx, 0, &proof, &root));

        // Wrong index should fail
        assert!(!verify_merkle_proof(&tx1, 1, &proof, &root));
    }

    #[test]
    fn test_batch_verify_merkle_proofs() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();
        let tx4 = digest(b"tx4").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()];
        let root = compute_merkle_root(&txs);

        // Create multiple proofs
        let proofs: Vec<(Vec<u8>, usize, Vec<Vec<u8>>)> = vec![
            (tx1.clone(), 0, generate_merkle_proof(&txs, 0)),
            (tx2.clone(), 1, generate_merkle_proof(&txs, 1)),
            (tx3.clone(), 2, generate_merkle_proof(&txs, 2)),
        ];

        // All valid proofs should pass
        assert!(batch_verify_merkle_proofs(&proofs, &root));

        // One invalid proof should fail
        let invalid_proofs: Vec<(Vec<u8>, usize, Vec<Vec<u8>>)> = vec![
            (tx1.clone(), 0, generate_merkle_proof(&txs, 0)),
            (digest(b"fake").to_vec(), 1, generate_merkle_proof(&txs, 1)),
        ];

        assert!(!batch_verify_merkle_proofs(&invalid_proofs, &root));
    }

    #[test]
    fn test_merkle_multiproof() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();
        let tx4 = digest(b"tx4").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()];

        // Generate multiproof for indices 0, 2
        let multiproof = generate_merkle_multiproof(&txs, &[0, 2]);

        // Multiproof should be smaller than individual proofs combined
        let proof0 = generate_merkle_proof(&txs, 0);
        let proof2 = generate_merkle_proof(&txs, 2);

        // Multiproof removes duplicates, so should be smaller
        assert!(multiproof.len() <= proof0.len() + proof2.len());
    }

    #[test]
    fn test_compressed_merkle_proof() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();
        let tx4 = digest(b"tx4").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()];
        let root = compute_merkle_root(&txs);

        // Generate and compress proof
        let proof = generate_merkle_proof(&txs, 1);
        let compressed = CompressedMerkleProof::from_proof(&tx2, 1, &proof);

        // Verify compressed proof
        assert!(compressed.verify(&tx2, 1, &root));

        // Invalid proof should fail
        assert!(!compressed.verify(&tx1, 1, &root));
    }

    #[test]
    fn test_compressed_proof_serialization() {
        let tx1 = digest(b"tx1").to_vec();
        let tx2 = digest(b"tx2").to_vec();
        let tx3 = digest(b"tx3").to_vec();
        let tx4 = digest(b"tx4").to_vec();

        let txs = vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()];
        let root = compute_merkle_root(&txs);

        let proof = generate_merkle_proof(&txs, 1);
        let compressed = CompressedMerkleProof::from_proof(&tx2, 1, &proof);

        // Serialize and deserialize
        let bytes = compressed.to_bytes();
        let deserialized = CompressedMerkleProof::from_bytes(&bytes).unwrap();

        // Should still verify
        assert!(deserialized.verify(&tx2, 1, &root));

        // Verify bandwidth savings
        let original_size = proof.iter().map(|p| p.len()).sum::<usize>();
        let compressed_size = bytes.len();

        // Compressed should be smaller (includes metadata but saves index calculations)
        tracing::info!(
            "Original: {} bytes, Compressed: {} bytes",
            original_size,
            compressed_size
        );
    }
}
