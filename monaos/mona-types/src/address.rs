/// Provides a way to get address length since it's a
/// platform-specific parameter.
use std::fmt;

/// Represents an address in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address([u8; 32]);

impl Address {
    pub fn new(bytes: [u8; 32]) -> Self {
        Address(bytes)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

