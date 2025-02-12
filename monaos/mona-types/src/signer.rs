use crate::address::Address;

/// Represents a signer that can authorize actions in the system.
/// Conceptually, this is a wrapper around an address that provides authorization.
#[derive(Debug, Clone)]
pub struct Signer {
    addr: Address,
}

impl Signer {
    /// Create a new signer with the given address
    pub fn new(addr: Address) -> Self {
        Signer { addr }
    }

    /// Borrow the address of the signer
    pub fn borrow_address(&self) -> &Address {
        &self.addr
    }

    /// Get the address of the signer
    pub fn address_of(&self) -> Address {
        self.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_operations() {
        let addr = Address::new([1u8; 32]);
        let signer = Signer::new(addr);
        
        // Test borrow_address
        assert_eq!(*signer.borrow_address(), addr);
        
        // Test address_of
        assert_eq!(signer.address_of(), addr);
    }
}