use sha2::{Sha256, Digest};
use sha3::Sha3_256;

/// Module which defines SHA hashes for byte vectors
#[derive(Debug)]
pub struct Hash;

impl Hash {
    /// Compute SHA2-256 hash of the data
    pub fn sha2_256(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Compute SHA3-256 hash of the data
    pub fn sha3_256(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_sha2_256() {
        let data = b"hello world";
        let hash = Hash::sha2_256(data);
        assert_eq!(
            hash,
            hex!("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
        );
    }

    #[test]
    fn test_sha3_256() {
        let data = b"hello world";
        let hash = Hash::sha3_256(data);
        assert_eq!(
            hash,
            hex!("644bcc7e564373040999aac89e7622f3ca71fba1d972fd94a31c3bfbf24e3938")
        );
    }
}