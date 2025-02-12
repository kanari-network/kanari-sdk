/// Provides a way to get address length since it's a
/// platform-specific parameter.
pub struct Address;

impl Address {
    /// Returns the length of an address in bytes.
    /// Should be converted to a platform-specific constant.
    /// Current implementation only works for Sui-compatible addresses.
    #[inline]
    pub const fn length() -> u64 {
        32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_length() {
        assert_eq!(Address::length(), 32);
    }
}