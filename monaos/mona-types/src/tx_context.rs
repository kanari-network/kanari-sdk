use std::vec::Vec;

/// Information about the transaction currently being executed.
#[derive(Debug, Clone)]
pub struct TxContext {
    /// The address of the user that signed the current transaction
    sender: [u8; 32],
    /// Hash of the current transaction
    tx_hash: Vec<u8>,
    /// The current epoch number
    epoch: u64,
    /// Timestamp that the epoch started at
    epoch_timestamp_ms: u64,
    /// Counter recording the number of fresh id's created
    ids_created: u64,
}

impl TxContext {
    /// Create a new TxContext instance
    pub fn new(
        sender: [u8; 32],
        tx_hash: Vec<u8>,
        epoch: u64,
        epoch_timestamp_ms: u64,
    ) -> Self {
        assert_eq!(tx_hash.len(), 32, "Transaction hash must be 32 bytes");
        Self {
            sender,
            tx_hash,
            epoch,
            epoch_timestamp_ms,
            ids_created: 0,
        }
    }

    /// Return the address of the user that signed the current transaction
    pub fn sender(&self) -> &[u8; 32] {
        &self.sender
    }

    /// Return the transaction digest (hash of transaction inputs)
    pub fn digest(&self) -> &[u8] {
        &self.tx_hash
    }

    /// Return the current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Return the epoch start time as a unix timestamp in milliseconds
    pub fn epoch_timestamp_ms(&self) -> u64 {
        self.epoch_timestamp_ms
    }

    /// Create a new unique object address
    pub fn fresh_object_address(&mut self) -> [u8; 32] {
        let id = self.derive_id(&self.tx_hash, self.ids_created);
        self.ids_created += 1;
        id
    }

    /// Return the number of IDs created
    pub fn ids_created(&self) -> u64 {
        self.ids_created
    }

    /// Derive a new ID by hashing the transaction hash and counter
    fn derive_id(&self, tx_hash: &[u8], ids_created: u64) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(tx_hash);
        hasher.update(ids_created.to_le_bytes());
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_context_basic() {
        let sender = [0u8; 32];
        let tx_hash = vec![0u8; 32];
        let ctx = TxContext::new(sender, tx_hash, 1, 1000);
        
        assert_eq!(ctx.epoch(), 1);
        assert_eq!(ctx.epoch_timestamp_ms(), 1000);
        assert_eq!(ctx.ids_created(), 0);
    }

    #[test]
    fn test_fresh_object_address() {
        let sender = [0u8; 32];
        let tx_hash = vec![0u8; 32];
        let mut ctx = TxContext::new(sender, tx_hash, 1, 1000);
        
        let addr1 = ctx.fresh_object_address();
        let addr2 = ctx.fresh_object_address();
        
        assert_ne!(addr1, addr2);
        assert_eq!(ctx.ids_created(), 2);
    }
}