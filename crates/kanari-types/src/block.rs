use bcs;
use serde::{Deserialize, Serialize};

/// Block header used to identify a block
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    /// Hash of previous block (empty for genesis)
    pub prev_hash: Vec<u8>,
    /// Sequential block number (starting at 1)
    pub block_number: u64,
    /// Unix timestamp seconds
    pub timestamp: u64,
    /// State hash (root) after applying block transactions
    pub state_hash: Vec<u8>,
}

impl BlockHeader {
    /// Serialize header deterministically (JSON used here)
    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).unwrap_or_default()
    }
}

/// Block structure holding header and optional signature
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    /// Optional signature bytes (e.g., from authority)
    pub signature: Option<Vec<u8>>,
}

impl Block {
    pub fn new(header: BlockHeader, signature: Option<Vec<u8>>) -> Self {
        Self { header, signature }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_bytes_roundtrip() {
        let h = BlockHeader {
            prev_hash: vec![],
            block_number: 1,
            timestamp: 1234567890,
            state_hash: vec![1, 2, 3],
        };
        let b = h.to_bytes();
        let parsed: BlockHeader = bcs::from_bytes(&b).unwrap();
        assert_eq!(parsed, h);
    }
}
