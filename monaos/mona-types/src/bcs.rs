use serde::Serialize;
use bcs::to_bytes as bcs_to_bytes;

/// Utility for converting a Rust value to its binary representation in BCS (Binary Canonical
/// Serialization). BCS is the binary encoding format used for serializing data structures.
pub struct BCS;

impl BCS {
    /// Convert a value to its binary representation in BCS format
    /// 
    /// # Arguments
    /// * `value` - Reference to a value that implements Serialize
    /// 
    /// # Returns
    /// * `Result<Vec<u8>>` - The BCS encoded bytes or an error
    pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, bcs::Error> {
        bcs_to_bytes(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestStruct {
        field: u64,
    }

    #[test]
    fn test_to_bytes() {
        let test_struct = TestStruct { field: 42 };
        let bytes = BCS::to_bytes(&test_struct).unwrap();
        assert!(!bytes.is_empty());
    }
}