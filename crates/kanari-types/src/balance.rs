// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Balance record structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BalanceRecord {
    pub value: u64,
}

impl BalanceRecord {
    /// Create a new balance record
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    /// Create a zero balance
    pub fn zero() -> Self {
        Self { value: 0 }
    }

    /// Check if balance is sufficient
    pub fn is_sufficient(&self, amount: u64) -> bool {
        self.value >= amount
    }

    /// Increase balance
    pub fn increase(&mut self, amount: u64) -> Result<()> {
        self.value = self
            .value
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Balance overflow"))?;
        Ok(())
    }

    /// Decrease balance
    pub fn decrease(&mut self, amount: u64) -> Result<()> {
        if self.value < amount {
            anyhow::bail!("Insufficient balance");
        }
        self.value -= amount;
        Ok(())
    }

    /// Return current value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Transfer amount from this balance into `to`
    pub fn transfer(&mut self, to: &mut BalanceRecord, amount: u64) -> Result<()> {
        self.decrease(amount)?;
        to.increase(amount)?;
        Ok(())
    }

    /// Merge another balance into this one (consumes other)
    pub fn merge(&mut self, other: BalanceRecord) {
        self.value = self.value + other.value;
    }

    /// Split off `amount` from this balance and return it as a new BalanceRecord
    pub fn split(&mut self, amount: u64) -> BalanceRecord {
        assert!(amount > 0, "zero amount");
        assert!(self.value >= amount, "insufficient balance");
        self.value = self.value - amount;
        BalanceRecord::new(amount)
    }

    /// Consume the balance and return its numeric value
    pub fn destroy(self) -> u64 {
        self.value
    }
}

/// Balance module constants and utilities
pub struct BalanceModule;

impl BalanceModule {
    pub const BALANCE_MODULE: &'static str = "balance";

    /// Get the module ID for kanari_system::balance
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::BALANCE_MODULE).context("Invalid balance module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in balance module
    pub fn function_names() -> BalanceFunctions {
        BalanceFunctions {
            zero: "zero",
            create: "create",
            value: "value",
            increase: "increase",
            decrease: "decrease",
            split: "split",
            merge: "merge",
            transfer: "transfer",
            has_sufficient: "has_sufficient",
            destroy: "destroy",
            new_supply: "new_supply",
            increase_supply: "increase_supply",
            destroy_supply: "destroy_supply",
            decrease_supply: "decrease_supply",
            supply_total: "supply_total",
        }
    }
}

/// Balance module function names
pub struct BalanceFunctions {
    pub zero: &'static str,
    pub create: &'static str,
    pub value: &'static str,
    pub increase: &'static str,
    pub decrease: &'static str,
    pub split: &'static str,
    pub merge: &'static str,
    pub transfer: &'static str,
    pub has_sufficient: &'static str,
    pub destroy: &'static str,
    pub new_supply: &'static str,
    pub increase_supply: &'static str,
    pub destroy_supply: &'static str,
    pub decrease_supply: &'static str,
    pub supply_total: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_creation() {
        let balance = BalanceRecord::new(1000);
        assert_eq!(balance.value, 1000);
    }

    #[test]
    fn test_balance_operations() {
        let mut balance = BalanceRecord::new(1000);
        balance.increase(500).unwrap();
        assert_eq!(balance.value, 1500);

        balance.decrease(300).unwrap();
        assert_eq!(balance.value, 1200);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut balance = BalanceRecord::new(100);
        let result = balance.decrease(200);
        assert!(result.is_err());
    }
}
