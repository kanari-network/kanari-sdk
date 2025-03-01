use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Gas schedule holding costs of VM operations with enhanced Move VM support
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
    /// Cost for contract execution overhead
    pub contract_execution_base_cost: u64,
    /// Custom operation costs
    pub custom_costs: BTreeMap<String, u64>,
    /// Move VM specific costs
    pub move_vm_costs: MoveVMCosts,
}

/// Move VM specific gas costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveVMCosts {
    /// Cost to load a Move module
    pub module_load_cost: u64,
    /// Cost to publish a Move module
    pub module_publish_cost: u64,
    /// Cost per byte for Move module storage
    pub module_byte_cost: u64,
    /// Base cost for function invocation
    pub function_call_base_cost: u64,
    /// Cost per type argument in function calls
    pub type_argument_cost: u64,
    /// Cost per argument in function calls
    pub argument_cost: u64,
    /// Cost for native function calls
    pub native_function_cost: u64,
    /// Cost for creating new resources
    pub resource_creation_cost: u64,
    /// Cost for resource field access
    pub resource_field_cost: u64,
}

impl Default for MoveVMCosts {
    fn default() -> Self {
        Self {
            module_load_cost: 200,
            module_publish_cost: 5000,
            module_byte_cost: 5,
            function_call_base_cost: 100,
            type_argument_cost: 10,
            argument_cost: 5,
            native_function_cost: 150,
            resource_creation_cost: 400,
            resource_field_cost: 10,
        }
    }
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
            contract_execution_base_cost: 500,
            custom_costs,
            move_vm_costs: MoveVMCosts::default(),
        }
    }
}

/// Configuration for gas metering with dynamic pricing
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
    /// Network congestion level (0-100)
    pub congestion_level: u8,
    /// Base fee for any transaction
    pub base_fee: u64,
    /// Priority fee multiplier
    pub priority_fee_multiplier: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            max_gas_per_tx: 1_000_000,
            gas_unit_scaling_factor: 1,
            min_price_per_gas_unit: 1,
            max_price_per_gas_unit: 10_000,
            congestion_level: 0,
            base_fee: 21_000, // Similar to Ethereum base fee
            priority_fee_multiplier: 1,
        }
    }
}

/// Gas meter for tracking gas usage during execution with refund support
#[derive(Debug)]
pub struct GasMeter {
    /// Remaining gas
    gas_left: u64,
    /// Gas used so far
    gas_used: u64,
    /// Potential gas to refund
    refundable_gas: u64,
    /// Gas schedule
    schedule: GasSchedule,
    /// Optional gas config for dynamic pricing
    config: Option<GasConfig>,
}

impl GasMeter {
    /// Create a new gas meter with maximum gas and schedule
    pub fn new(max_gas: u64, schedule: GasSchedule) -> Self {
        Self {
            gas_left: max_gas,
            gas_used: 0,
            refundable_gas: 0,
            schedule,
            config: None,
        }
    }

    /// Create a new gas meter with dynamic pricing configuration
    pub fn new_with_config(max_gas: u64, schedule: GasSchedule, config: GasConfig) -> Self {
        Self {
            gas_left: max_gas,
            gas_used: 0,
            refundable_gas: 0,
            schedule,
            config: Some(config),
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

    /// Add gas to refundable pool (for operations like storage release)
    pub fn add_refundable_gas(&mut self, amount: u64) {
        self.refundable_gas += amount;
    }

    /// Apply refund at the end of transaction (limited to a portion of used gas)
    pub fn apply_refund(&mut self) -> u64 {
        // Limit refund to half of gas used, similar to EIP-3529
        let max_refund = self.gas_used / 2;
        let actual_refund = std::cmp::min(self.refundable_gas, max_refund);
        self.gas_left += actual_refund;
        self.gas_used -= actual_refund;
        self.refundable_gas = 0;
        actual_refund
    }

    /// Charge gas for a Move VM module operation
    pub fn charge_move_module_op(&mut self, module_bytes: usize, is_publish: bool) -> Result<(), GasError> {
        let base_cost = if is_publish {
            self.schedule.move_vm_costs.module_publish_cost
        } else {
            self.schedule.move_vm_costs.module_load_cost
        };
        let byte_cost = module_bytes as u64 * self.schedule.move_vm_costs.module_byte_cost;
        self.deduct_gas(base_cost + byte_cost)
    }

    /// Charge gas for a Move function call
    pub fn charge_move_function_call(
        &mut self, 
        num_args: usize, 
        num_type_args: usize,
        is_native: bool,
    ) -> Result<(), GasError> {
        let base_cost = self.schedule.move_vm_costs.function_call_base_cost;
        let args_cost = num_args as u64 * self.schedule.move_vm_costs.argument_cost;
        let type_args_cost = num_type_args as u64 * self.schedule.move_vm_costs.type_argument_cost;
        let native_cost = if is_native { self.schedule.move_vm_costs.native_function_cost } else { 0 };
        
        self.deduct_gas(base_cost + args_cost + type_args_cost + native_cost)
    }

    /// Calculate gas price based on current network conditions
    pub fn calculate_effective_gas_price(&self, user_gas_price: u64) -> Result<u64, GasError> {
        match &self.config {
            Some(config) => {
                if user_gas_price < config.min_price_per_gas_unit {
                    return Err(GasError::GasPriceTooLow(user_gas_price));
                }
                
                if user_gas_price > config.max_price_per_gas_unit {
                    return Err(GasError::GasPriceTooHigh(user_gas_price));
                }

                // Calculate congestion factor (1.0 to 2.0)
                let congestion_factor = 1.0 + (config.congestion_level as f64 / 100.0);
                
                // Apply congestion to base fee
                let adjusted_base_fee = (config.base_fee as f64 * congestion_factor) as u64;
                
                // Final price is adjusted base fee + user's priority fee
                let priority_fee = user_gas_price * config.priority_fee_multiplier;
                Ok(adjusted_base_fee + priority_fee)
            },
            None => {
                // Without config, just return the user price
                Ok(user_gas_price)
            }
        }
    }

    /// Get remaining gas
    pub fn gas_left(&self) -> u64 {
        self.gas_left
    }

    /// Get used gas
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }
    
    /// Get refundable gas
    pub fn refundable_gas(&self) -> u64 {
        self.refundable_gas
    }
    
    /// Get gas schedule
    pub fn schedule(&self) -> &GasSchedule {
        &self.schedule
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
    #[error("Operation not allowed: {0}")]
    OperationNotAllowed(String),
}