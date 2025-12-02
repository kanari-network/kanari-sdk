//! String Module - Move stdlib string implementation
//!
//! The string module defines the String type which represents UTF8 encoded strings.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// UTF-8 string structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Utf8String {
    pub bytes: Vec<u8>,
}

impl Utf8String {
    /// Create new UTF-8 string from bytes
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        // Verify valid UTF-8
        String::from_utf8(bytes.clone()).context("Invalid UTF-8")?;
        Ok(Self { bytes })
    }

    /// Create from string slice
    pub fn from_str(s: &str) -> Self {
        Self {
            bytes: s.as_bytes().to_vec(),
        }
    }

    /// Convert to Rust String
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.bytes.clone()).context("Invalid UTF-8 in string")
    }

    /// Get length in bytes
    pub fn length(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Append another string
    pub fn append(&mut self, other: &Utf8String) {
        self.bytes.extend_from_slice(&other.bytes);
    }

    /// Get substring (by byte indices)
    pub fn substring(&self, start: usize, end: usize) -> Result<Self> {
        if end > self.bytes.len() || start > end {
            anyhow::bail!("Index out of range");
        }
        Self::new(self.bytes[start..end].to_vec())
    }
}

impl From<String> for Utf8String {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

impl From<&str> for Utf8String {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

/// String module constants and utilities
pub struct StringModule;

impl StringModule {
    pub const MODULE_NAME: &'static str = "string";

    /// Error: invalid UTF-8 encoding
    pub const EINVALID_UTF8: u64 = 1;
    /// Error: index out of range
    pub const EINVALID_INDEX: u64 = 2;

    /// Get the module ID for std::string
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::STD_ADDRESS)
            .context("Invalid std address")?;

        let module_name =
            Identifier::new(Self::MODULE_NAME).context("Invalid string module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names
    pub fn function_names() -> StringFunctions {
        StringFunctions {
            utf8: "utf8",
            try_utf8: "try_utf8",
            bytes: "bytes",
            is_empty: "is_empty",
            length: "length",
            append: "append",
            substring: "substring",
        }
    }
}

/// String module function names
pub struct StringFunctions {
    pub utf8: &'static str,
    pub try_utf8: &'static str,
    pub bytes: &'static str,
    pub is_empty: &'static str,
    pub length: &'static str,
    pub append: &'static str,
    pub substring: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_string_creation() {
        let s = Utf8String::from_str("Hello, World!");
        assert_eq!(s.length(), 13);
        assert!(!s.is_empty());
        assert_eq!(s.to_string().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_utf8_with_unicode() {
        let s = Utf8String::from_str("สวัสดี");
        assert!(s.length() > 0);
        assert_eq!(s.to_string().unwrap(), "สวัสดี");
    }

    #[test]
    fn test_string_append() {
        let mut s1 = Utf8String::from_str("Hello");
        let s2 = Utf8String::from_str(", World!");
        s1.append(&s2);
        assert_eq!(s1.to_string().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_substring() {
        let s = Utf8String::from_str("Hello, World!");
        let sub = s.substring(0, 5).unwrap();
        assert_eq!(sub.to_string().unwrap(), "Hello");
    }
}
