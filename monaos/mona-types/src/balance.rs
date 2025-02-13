use std::marker::PhantomData;
use crate::tx_context::TxContext;

/// Error codes
#[derive(Debug)]
pub enum BalanceError {
    NonZero,
    Overflow,
    NotEnough,
    NotSystemAddress,
}

/// A Supply of T. Used for minting and burning.
#[derive(Debug)]
pub struct Supply<T> {
    value: u64,
    _phantom: PhantomData<T>,
}

/// Storable balance - represents a stored amount of a token type
#[derive(Debug)]
pub struct Balance<T> {
    value: u64,
    _phantom: PhantomData<T>,
}

impl<T> Supply<T> {
    /// Create a new supply for type T
    pub fn create_supply(_witness: T) -> Self {
        Self {
            value: 0,
            _phantom: PhantomData,
        }
    }

    /// Get the supply value
    pub fn supply_value(&self) -> u64 {
        self.value
    }

    /// Increase supply by value and create a new Balance<T>
    pub fn increase_supply(&mut self, value: u64) -> Result<Balance<T>, BalanceError> {
        if value >= (u64::MAX - self.value) {
            return Err(BalanceError::Overflow);
        }
        self.value += value;
        Ok(Balance {
            value,
            _phantom: PhantomData,
        })
    }

    /// Burn a Balance<T> and decrease Supply<T>
    pub fn decrease_supply(&mut self, balance: Balance<T>) -> Result<u64, BalanceError> {
        let value = balance.value;
        if self.value < value {
            return Err(BalanceError::Overflow);
        }
        self.value -= value;
        Ok(value)
    }
}

impl<T> Balance<T> {
    /// Get the amount stored in a Balance
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Create a zero Balance
    pub fn zero() -> Self {
        Self {
            value: 0,
            _phantom: PhantomData,
        }
    }

    /// Join two balances together
    pub fn join(&mut self, other: Balance<T>) -> u64 {
        self.value += other.value;
        self.value
    }

    /// Split a Balance and take a sub balance from it
    pub fn split(&mut self, value: u64) -> Result<Balance<T>, BalanceError> {
        if self.value < value {
            return Err(BalanceError::NotEnough);
        }
        self.value -= value;
        Ok(Balance {
            value,
            _phantom: PhantomData,
        })
    }

    /// Withdraw all balance
    pub fn withdraw_all(&mut self) -> Balance<T> {
        let value = self.value;
        self.value = 0;
        Balance {
            value,
            _phantom: PhantomData,
        }
    }

    /// Destroy a zero Balance
    pub fn destroy_zero(self) -> Result<(), BalanceError> {
        if self.value != 0 {
            return Err(BalanceError::NonZero);
        }
        Ok(())
    }
}

// System-level functions
impl<T> Balance<T> {
    /// Create staking rewards (system only)
    fn create_staking_rewards(value: u64, ctx: &TxContext) -> Result<Balance<T>, BalanceError> {
        if ctx.sender() != &[0u8; 32] {
            return Err(BalanceError::NotSystemAddress);
        }
        Ok(Balance {
            value,
            _phantom: PhantomData,
        })
    }

    /// Destroy storage rebates (system only)
    fn destroy_storage_rebates(self, ctx: &TxContext) -> Result<(), BalanceError> {
        if ctx.sender() != &[0u8; 32] {
            return Err(BalanceError::NotSystemAddress);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dummy token type for testing
    struct TestToken;

    #[test]
    fn test_supply_operations() {
        let mut supply = Supply::create_supply(TestToken);
        assert_eq!(supply.supply_value(), 0);

        let balance = supply.increase_supply(100).unwrap();
        assert_eq!(balance.value(), 100);
        assert_eq!(supply.supply_value(), 100);

        let value = supply.decrease_supply(balance).unwrap();
        assert_eq!(value, 100);
        assert_eq!(supply.supply_value(), 0);
    }

    #[test]
    fn test_balance_operations() {
        let mut balance = Balance::<TestToken>::zero();
        assert_eq!(balance.value(), 0);

        let mut supply = Supply::create_supply(TestToken);
        let balance2 = supply.increase_supply(50).unwrap();
        balance.join(balance2);
        assert_eq!(balance.value(), 50);

        let split = balance.split(30).unwrap();
        assert_eq!(split.value(), 30);
        assert_eq!(balance.value(), 20);
    }
}