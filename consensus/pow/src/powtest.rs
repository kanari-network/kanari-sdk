#[cfg(test)]
mod tests {
    use crate::{adjust_difficulty, HashAlgorithm, PoWBlock, proof_of_work, Sha256Algorithm};

    #[test]
    fn test_sha256_algorithm() {
        let hasher = Sha256Algorithm;
        let input = b"hello world";
        let expected_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(hasher.hash(input), expected_hash);
    }

    #[test]
    fn test_pow_block_new() {
        let hasher = Sha256Algorithm;
        let block = PoWBlock::new(0, 0, "data".to_string(), "prev_hash".to_string(), hasher);
        assert_eq!(block.index, 0);
        assert_eq!(block.timestamp, 0);
        assert_eq!(block.data, "data");
        assert_eq!(block.prev_block_hash, "prev_hash");
        assert_eq!(block.hash, "");
        assert_eq!(block.nonce, 0);
    }

    #[test]
    fn test_pow_block_calculate_hash() {
        let hasher = Sha256Algorithm;
        let block = PoWBlock::new(0, 0, "data".to_string(), "prev_hash".to_string(), hasher);
        let expected_hash = "f543996656106bce99e10dcbe6945f63569165d406d850ddcf903a6013830eba";
        assert_eq!(block.calculate_hash(), expected_hash);
    }

    #[test]
    fn test_proof_of_work() {
        let hasher = Sha256Algorithm;
        let mut block = PoWBlock::new(0, 0, "data".to_string(), "prev_hash".to_string(), hasher);
        let difficulty = 2;
        proof_of_work(&mut block, difficulty);
        assert!(block.hash.starts_with(&"0".repeat(difficulty)));
    }

    #[test]
    fn test_adjust_difficulty() {
        assert_eq!(adjust_difficulty(1), 4);
        assert_eq!(adjust_difficulty(2), 5);
        assert_eq!(adjust_difficulty(10), 13);
    }
}