#[cfg(test)]
mod tests {
    use crate::{proof_of_work, Blake3Algorithm, PoWBlock};

    #[test]
    fn test_calculate_hash() {
        let hasher = Blake3Algorithm;
        let block = PoWBlock::new(0, "Test Data".to_string(), "0000".to_string(), "Validator".to_string(), hasher, 1);
        let hash = block.calculate_hash();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_mine() {
        let hasher = Blake3Algorithm;
        let mut block = PoWBlock::new(0, "Test Data".to_string(), "0000".to_string(), "Validator".to_string(), hasher, 1);
        block.mine();
        assert!(block.hash.starts_with("0"));
    }

    #[test]
    fn test_proof_of_work() {
        let hasher = Blake3Algorithm;
        let mut block = PoWBlock::new(0, "Test Data".to_string(), "0000".to_string(), "Validator".to_string(), hasher, 1);
        proof_of_work(&mut block);
        assert!(block.hash.starts_with("0"));
    }
}