use crate::ascii::{self, String as AsciiString};
use crate::address;
use crate::vector::Vector;

/// ASCII Character codes
const ASCII_COLON: u8 = 58;
const ASCII_V: u8 = 118;
const ASCII_E: u8 = 101;
const ASCII_C: u8 = 99;
const ASCII_T: u8 = 116;
const ASCII_O: u8 = 111;
const ASCII_R: u8 = 114;

/// Error type for non-module type operations
#[derive(Debug)]
pub struct NonModuleTypeError;

/// Represents a type name in the system
#[derive(Debug, Clone)]
pub struct TypeName {
    /// String representation of the type
    name: AsciiString,
}

impl TypeName {
    /// Create a new TypeName from an ASCII string
    pub fn new(name: AsciiString) -> Self {
        TypeName { name }
    }

    /// Check if the type is primitive
    pub fn is_primitive(&self) -> bool {
        let bytes = self.name.as_bytes();
        
        bytes == b"bool" ||
        bytes == b"u8" ||
        bytes == b"u16" ||
        bytes == b"u32" ||
        bytes == b"u64" ||
        bytes == b"u128" ||
        bytes == b"u256" ||
        bytes == b"address" ||
        (bytes.len() >= 6 &&
            bytes[0] == ASCII_V &&
            bytes[1] == ASCII_E &&
            bytes[2] == ASCII_C &&
            bytes[3] == ASCII_T &&
            bytes[4] == ASCII_O &&
            bytes[5] == ASCII_R)
    }

    /// Borrow the underlying string
    pub fn borrow_string(&self) -> &AsciiString {
        &self.name
    }

    /// Get the address part of a type name
    pub fn get_address(&self) -> Result<AsciiString, NonModuleTypeError> {
        if self.is_primitive() {
            return Err(NonModuleTypeError);
        }

        // Base16 (string) representation of an address has 2 symbols per byte
        let len = address::Address::new([0; 32]).to_string().len() - 2; // subtract 2 for "0x" prefix
        let addr_bytes = self.name.as_bytes()[..len].to_vec();
        
        AsciiString::new(addr_bytes).map_err(|_| NonModuleTypeError)
    }

    /// Get the module name part
    pub fn get_module(&self) -> Result<AsciiString, NonModuleTypeError> {
        if self.is_primitive() {
            return Err(NonModuleTypeError);
        }

        let bytes = self.name.as_bytes();
        let start = address::Address::new([0; 32]).to_string().len() - 2 + 2; // addr len + "::"
        let mut module_bytes = Vec::new();

        for &byte in bytes[start..].iter() {
            if byte == ASCII_COLON {
                break;
            }
            module_bytes.push(byte);
        }

        AsciiString::new(module_bytes).map_err(|_| NonModuleTypeError)
    }

    /// Convert into the underlying ASCII string
    pub fn into_string(self) -> AsciiString {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        let bool_type = TypeName::new(AsciiString::new(b"bool".to_vec()).unwrap());
        assert!(bool_type.is_primitive());

        let vector_type = TypeName::new(AsciiString::new(b"vector".to_vec()).unwrap());
        assert!(vector_type.is_primitive());
    }

    #[test]
    fn test_complex_type() {
        let type_str = b"000000000000000000000000000000aa::module::Type".to_vec();
        let complex_type = TypeName::new(AsciiString::new(type_str).unwrap());
        
        assert!(!complex_type.is_primitive());
        assert!(complex_type.get_module().is_ok());
    }
}