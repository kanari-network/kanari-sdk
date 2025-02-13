use serde::{Deserialize, Serialize};

use bcs::to_bytes;
use thiserror::Error;

/// The hardcoded IDs for singleton objects
pub const KARI_SYSTEM_STATE_OBJECT_ID: [u8; 32] = [0x5; 32];
pub const KARI_CLOCK_OBJECT_ID: [u8; 32] = [0x6; 32];
pub const KARI_AUTHENTICATOR_STATE_ID: [u8; 32] = [0x7; 32];
pub const KARI_RANDOM_ID: [u8; 32] = [0x8; 32];
pub const KARI_DENY_LIST_OBJECT_ID: [u8; 32] = [
    0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // First 8 bytes
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Next 8 bytes
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Next 8 bytes
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Final 8 bytes
];

/// An object ID used to reference Kari Objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ID {
    bytes: [u8; 32], // Using 32 bytes for address
}

/// Globally unique ID that must be the first field of any Kari Object
#[derive(Debug, Serialize, Deserialize)]
pub struct UID {
    id: ID,
}

#[derive(Debug, Error)]
pub enum ObjectError {
    #[error("Invalid ID length, expected 32 bytes")]
    InvalidLength,
    #[error("BCS serialization error: {0}")]
    SerializationError(#[from] bcs::Error),
}

impl ID {
    /// Create an ID from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectError> {
        if bytes.len() != 32 {
            return Err(ObjectError::InvalidLength);
        }
        let mut id_bytes = [0u8; 32];
        id_bytes.copy_from_slice(bytes);
        Ok(Self { bytes: id_bytes })
    }

    /// Get the raw bytes of the ID
    pub fn to_bytes(&self) -> Result<Vec<u8>, ObjectError> {
        bcs::to_bytes(&self.bytes).map_err(ObjectError::SerializationError)
    }

    /// Get the address bytes directly
    pub fn address_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl UID {
    /// Create a new UID from a context
    pub fn new(ctx: &mut TxContext) -> Self {
        Self {
            id: ID {
                bytes: ctx.fresh_object_address(),
            },
        }
    }

    /// Create UID from hash
    pub(crate) fn new_from_hash(bytes: [u8; 32]) -> Self {
        record_new_uid(&bytes);
        Self { id: ID { bytes } }
    }

    /// Get the inner ID
    pub fn as_id(&self) -> &ID {
        &self.id
    }

    /// Get the raw bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        to_bytes(&self.id.bytes).unwrap_or_default()
    }

    /// Get the address bytes
    pub fn address_bytes(&self) -> &[u8; 32] {
        &self.id.bytes
    }

    /// Convert the UID to an address represented as [u8; 32]
    pub fn to_address(&self) -> [u8; 32] {
        *self.address_bytes()
    }
}

/// Transaction context for generating fresh object addresses
#[derive(Default)]
pub struct TxContext {
    next_id: u64,
}

impl TxContext {
    fn fresh_object_address(&mut self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let id_bytes = self.next_id.to_le_bytes();
        bytes[0..8].copy_from_slice(&id_bytes);
        self.next_id += 1;
        bytes
    }
}

// Internal function to record new UIDs
fn record_new_uid(_bytes: &[u8; 32]) {
    // Implementation would track UIDs in production
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uid_creation() {
        let mut ctx = TxContext::default();
        let uid = UID::new(&mut ctx);
        assert_ne!(uid.to_bytes(), vec![0; 32]);
    }

    #[test]
    fn test_id_from_bytes() {
        let bytes = [1u8; 32];
        let id = ID::from_bytes(&bytes).unwrap();
        assert_eq!(id.address_bytes(), &bytes);
    }

    #[test]
    fn test_uid_from_hash() {
        let hash = [2u8; 32];
        let uid = UID::new_from_hash(hash);
        assert_eq!(uid.address_bytes(), &hash);
    }

    #[test]
    fn test_uid_to_address() {
        let hash = [3u8; 32];
        let uid = UID::new_from_hash(hash);
        assert_eq!(uid.to_address(), hash);
        
        // Test that to_address matches address_bytes
        assert_eq!(&uid.to_address(), uid.address_bytes());
    }
}
