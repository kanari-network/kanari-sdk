use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Gas schedule holding costs of VM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasSchedule {
    /// Cost of executing a single instruction
    pub instruction_cost: u64,
    /// Cost of reading from storage
    pub read_cost: u64,
    /// Cost of writing to storage
    pub write_cost: u64,
    /// Cost per byte for storage operations
    pub storage_byte_cost: u64,
    /// Cost of creating a new account
    pub account_creation_cost: u64,
    /// Custom operation costs
    pub custom_costs: BTreeMap<String, u64>,
}

impl Default for GasSchedule {
    fn default() -> Self {
        let mut custom_costs = BTreeMap::new();
        custom_costs.insert("move_publish".to_string(), 1000);
        custom_costs.insert("move_upgrade".to_string(), 2000);
        custom_costs.insert("max_gas_per_tx".to_string(), 1_000_000);

        Self {
            instruction_cost: 1,
            read_cost: 100,
            write_cost: 300,
            storage_byte_cost: 1,
            account_creation_cost: 1000,
            custom_costs,
        }
    }
}

/// Configuration for gas metering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConfig {
    /// Maximum gas units per transaction
    pub max_gas_per_tx: u64,
    /// Global gas unit price multiplier
    pub gas_unit_scaling_factor: u64,
    /// Minimum price per gas unit
    pub min_price_per_gas_unit: u64,
    /// Maximum price per gas unit
    pub max_price_per_gas_unit: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            max_gas_per_tx: 1_000_000,
            gas_unit_scaling_factor: 1,
            min_price_per_gas_unit: 1,
            max_price_per_gas_unit: 10_000,
        }
    }
}

/// Gas meter for tracking gas usage during execution
#[derive(Debug)]
pub struct GasMeter {
    gas_left: u64,
    gas_used: u64,
    schedule: GasSchedule,
}

impl GasMeter {
    pub fn new(max_gas: u64, schedule: GasSchedule) -> Self {
        Self {
            gas_left: max_gas,
            gas_used: 0,
            schedule,
        }
    }

    /// Deduct gas for an operation
    pub fn deduct_gas(&mut self, amount: u64) -> Result<(), GasError> {
        if self.gas_left < amount {
            return Err(GasError::OutOfGas {
                requested: amount,
                available: self.gas_left,
            });
        }
        self.gas_left -= amount;
        self.gas_used += amount;
        Ok(())
    }

    /// Charge gas for storage operations based on size
    pub fn charge_storage_op(&mut self, bytes: usize, is_write: bool) -> Result<(), GasError> {
        let base_cost = if is_write {
            self.schedule.write_cost
        } else {
            self.schedule.read_cost
        };
        let byte_cost = bytes as u64 * self.schedule.storage_byte_cost;
        self.deduct_gas(base_cost + byte_cost)
    }

    /// Get remaining gas
    pub fn gas_left(&self) -> u64 {
        self.gas_left
    }

    /// Get used gas
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }
}

/// Gas related errors
#[derive(Debug, thiserror::Error)]
pub enum GasError {
    #[error("Out of gas: requested {requested} units but only {available} available")]
    OutOfGas {
        requested: u64,
        available: u64,
    },
    #[error("Gas price {0} exceeds maximum allowed")]
    GasPriceTooHigh(u64),
    #[error("Gas price {0} below minimum required")]
    GasPriceTooLow(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_deduction() {
        let schedule = GasSchedule::default();
        let mut meter = GasMeter::new(1000, schedule);

        assert!(meter.deduct_gas(500).is_ok());
        assert_eq!(meter.gas_left(), 500);
        assert_eq!(meter.gas_used(), 500);

        assert!(meter.deduct_gas(600).is_err());
        assert_eq!(meter.gas_left(), 500); // Should not change on error
    }

    #[test]
    fn test_storage_op_charging() {
        let schedule = GasSchedule::default();
        let mut meter = GasMeter::new(1000, schedule.clone());

        // Test read operation
        assert!(meter.charge_storage_op(100, false).is_ok());
        let expected_cost = schedule.read_cost + (100 * schedule.storage_byte_cost) as u64;
        assert_eq!(meter.gas_used(), expected_cost);

        // Test write operation
        assert!(meter.charge_storage_op(50, true).is_ok());
        let additional_cost = schedule.write_cost + (50 * schedule.storage_byte_cost) as u64;
        assert_eq!(meter.gas_used(), expected_cost + additional_cost);
    }
}