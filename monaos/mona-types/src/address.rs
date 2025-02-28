use std::fmt;
use std::str::FromStr;
use hex::FromHex;
use std::convert::TryFrom;
use serde::{Serialize, Deserialize}; // Import Serialize and Deserialize

/// Represents an address in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address([u8; Address::LENGTH]);

impl Address {
    /// The number of bytes in an address
    pub const LENGTH: usize = 32;
    
    /// Creates a new Address from raw bytes
    pub const fn new(bytes: [u8; Self::LENGTH]) -> Self {
        Address(bytes)
    }

    /// Zero address constant
    pub const ZERO: Self = Self([0u8; Self::LENGTH]);

    /// Returns the underlying bytes
    pub fn to_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
    
    /// Convert address to vector of bytes
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
    
    /// Consume address and return the raw bytes
    pub fn into_bytes(self) -> [u8; Self::LENGTH] {
        self.0
    }

    /// Create an address from a hex string
    pub fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, AddressParseError> {
        <[u8; Self::LENGTH]>::from_hex(hex)
            .map_err(|_| AddressParseError)
            .map(Self)
    }

    /// Convert address to hex string without 0x prefix
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Convert address to hex string with 0x prefix
    pub fn to_hex_literal(&self) -> String {
        format!("0x{}", self.to_hex())
    }
    
    /// Parse a hex literal (with 0x prefix)
    pub fn from_hex_literal(literal: &str) -> Result<Self, AddressParseError> {
        if !literal.starts_with("0x") {
            return Err(AddressParseError);
        }

        let hex_len = literal.len() - 2;

        // Pad if too short
        if hex_len < Self::LENGTH * 2 {
            let mut hex_str = String::with_capacity(Self::LENGTH * 2);
            for _ in 0..Self::LENGTH * 2 - hex_len {
                hex_str.push('0');
            }
            hex_str.push_str(&literal[2..]);
            Self::from_hex(hex_str)
        } else {
            Self::from_hex(&literal[2..])
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl fmt::LowerHex for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; Address::LENGTH]> for Address {
    fn from(bytes: [u8; Address::LENGTH]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = AddressParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; Self::LENGTH]>::try_from(bytes)
            .map_err(|_| AddressParseError)
            .map(Self)
    }
}

impl FromStr for Address {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(address) = Address::from_hex_literal(s) {
            Ok(address)
        } else {
            Self::from_hex(s)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddressParseError;

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unable to parse Address (must be hex string of length {})",
            Address::LENGTH
        )
    }
}

impl std::error::Error for AddressParseError {}

