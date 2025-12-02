//! ASCII Module - Move stdlib ascii implementation
//!
//! The ASCII module defines basic string and char newtypes that verify
//! characters are valid ASCII.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// ASCII string structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AsciiString {
    pub bytes: Vec<u8>,
}

impl AsciiString {
    /// Create new ASCII string from bytes
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        // Verify all bytes are valid ASCII
        if !bytes.iter().all(|&b| b < 128) {
            anyhow::bail!("Invalid ASCII character");
        }
        Ok(Self { bytes })
    }

    /// Create from string slice
    pub fn from_str(s: &str) -> Result<Self> {
        Self::new(s.as_bytes().to_vec())
    }

    /// Convert to string
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.bytes.clone()).context("Invalid UTF-8 in ASCII string")
    }

    /// Get length
    pub fn length(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Check if all characters are printable
    pub fn all_characters_printable(&self) -> bool {
        self.bytes.iter().all(|&b| (32..=126).contains(&b))
    }
}

/// ASCII Char structure
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiChar {
    pub byte: u8,
}

impl AsciiChar {
    /// Create new ASCII char
    pub fn new(byte: u8) -> Result<Self> {
        if byte >= 128 {
            anyhow::bail!("Invalid ASCII character: {}", byte);
        }
        Ok(Self { byte })
    }

    /// Check if printable
    pub fn is_printable(&self) -> bool {
        (32..=126).contains(&self.byte)
    }
}

/// ASCII module constants and utilities
pub struct AsciiModule;

impl AsciiModule {
    pub const MODULE_NAME: &'static str = "ascii";

    /// Error: invalid ASCII character
    pub const EINVALID_ASCII_CHARACTER: u64 = 0x10000;

    /// Get the module ID for std::ascii
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid ascii module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> AsciiFunctions {
        AsciiFunctions {
            char: "char",
            string: "string",
            is_valid_char: "is_valid_char",
            length: "length",
            is_empty: "is_empty",
            all_characters_printable: "all_characters_printable",
        }
    }
}

/// ASCII module function names
pub struct AsciiFunctions {
    pub char: &'static str,
    pub string: &'static str,
    pub is_valid_char: &'static str,
    pub length: &'static str,
    pub is_empty: &'static str,
    pub all_characters_printable: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_string_creation() {
        let s = AsciiString::from_str("Hello").unwrap();
        assert_eq!(s.length(), 5);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_invalid_ascii() {
        let result = AsciiString::new(vec![200]);
        assert!(result.is_err());
    }

    #[test]
    fn test_printable() {
        let s = AsciiString::from_str("ABC").unwrap();
        assert!(s.all_characters_printable());
    }
}
