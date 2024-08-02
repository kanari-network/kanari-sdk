use crate::{Blake3Algorithm, PoSBlock, proof_of_stake};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pos_block_creation() {
        let hasher = Blake3Algorithm;
        let block = PoSBlock::new(0, "data".to_string(), "prev_hash".to_string(), "validator".to_string(), hasher);
        assert_eq!(block.index, 0);
        assert_eq!(block.data, "data");
        assert_eq!(block.prev_block_hash, "prev_hash");
        assert_eq!(block.validator, "validator");
    }

    #[test]
    fn test_pos_block_hashing() {
        let hasher = Blake3Algorithm;
        let mut block = PoSBlock::new(0, "data".to_string(), "prev_hash".to_string(), "validator".to_string(), hasher);
        proof_of_stake(&mut block);
        assert!(!block.hash.is_empty());
    }

    #[test]
    fn test_pos_block_hash_consistency() {
        let hasher = Blake3Algorithm;
        let mut block1 = PoSBlock::new(0, "data".to_string(), "prev_hash".to_string(), "validator".to_string(), hasher);
        let mut block2 = PoSBlock::new(0, "data".to_string(), "prev_hash".to_string(), "validator".to_string(), Blake3Algorithm);
        proof_of_stake(&mut block1);
        proof_of_stake(&mut block2);
        assert_eq!(block1.hash, block2.hash);
    }
}