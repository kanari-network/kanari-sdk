use serde::{Deserialize, Serialize};

/// Gas configuration and pricing for the Kanari blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConfig {
    /// Base gas price per unit (in Mist)
    pub base_price: u64,

    /// Maximum gas units per transaction
    pub max_gas_per_tx: u64,

    /// Maximum gas units per block
    pub max_gas_per_block: u64,

    /// Minimum gas price (in Mist)
    pub min_gas_price: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            base_price: 1000,              // 1,000 Mist per gas unit
            max_gas_per_tx: 1_000_000,     // 1M gas per transaction
            max_gas_per_block: 10_000_000, // 10M gas per block
            min_gas_price: 100,            // 100 Mist minimum
        }
    }
}

/// Gas costs for different operations
#[derive(Debug, Clone, Copy)]
pub enum GasOperation {
    /// Transfer native tokens
    Transfer,
    /// Publish a Move module
    PublishModule { module_size: usize },
    /// Execute a Move function
    ExecuteFunction { complexity: u32 },
    /// Call a contract function
    ContractCall { function_name_len: usize },
    /// Deploy a contract with metadata
    ContractDeployment {
        module_size: usize,
        metadata_size: usize,
    },
    /// Query contract information
    ContractQuery,
    /// Create new account
    CreateAccount,
    /// Update account state
    UpdateAccount,
}

impl GasOperation {
    /// Calculate gas units required for this operation
    pub fn gas_units(&self) -> u64 {
        match self {
            GasOperation::Transfer => 21_000,
            GasOperation::PublishModule { module_size } => {
                // Base cost + per-byte cost
                50_000 + (*module_size as u64 * 10)
            }
            GasOperation::ExecuteFunction { complexity } => {
                // Base cost + complexity multiplier
                30_000 + (*complexity as u64 * 1_000)
            }
            GasOperation::ContractCall { function_name_len } => {
                // Base cost for contract call + name length overhead
                35_000 + (*function_name_len as u64 * 100)
            }
            GasOperation::ContractDeployment {
                module_size,
                metadata_size,
            } => {
                // Higher cost for full contract deployment with registry
                60_000 + (*module_size as u64 * 10) + (*metadata_size as u64 * 5)
            }
            GasOperation::ContractQuery => 1_000,
            GasOperation::CreateAccount => 25_000,
            GasOperation::UpdateAccount => 5_000,
        }
    }

    /// Get operation name for logging
    pub fn name(&self) -> &str {
        match self {
            GasOperation::Transfer => "Transfer",
            GasOperation::PublishModule { .. } => "PublishModule",
            GasOperation::ExecuteFunction { .. } => "ExecuteFunction",
            GasOperation::ContractCall { .. } => "ContractCall",
            GasOperation::ContractDeployment { .. } => "ContractDeployment",
            GasOperation::ContractQuery => "ContractQuery",
            GasOperation::CreateAccount => "CreateAccount",
            GasOperation::UpdateAccount => "UpdateAccount",
        }
    }
}

/// Gas meter for tracking gas usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasMeter {
    /// Gas units used
    pub gas_used: u64,

    /// Gas price per unit (in Mist)
    pub gas_price: u64,

    /// Maximum gas allowed
    pub gas_limit: u64,
}

impl GasMeter {
    pub fn new(gas_limit: u64, gas_price: u64) -> Self {
        Self {
            gas_used: 0,
            gas_price,
            gas_limit,
        }
    }

    /// Consume gas for an operation
    pub fn consume(&mut self, gas_units: u64) -> Result<(), GasError> {
        let new_usage = self
            .gas_used
            .checked_add(gas_units)
            .ok_or(GasError::Overflow)?;

        if new_usage > self.gas_limit {
            return Err(GasError::OutOfGas {
                required: new_usage,
                limit: self.gas_limit,
            });
        }

        self.gas_used = new_usage;
        Ok(())
    }

    /// Calculate total gas cost in Mist
    pub fn total_cost(&self) -> u64 {
        self.gas_used.saturating_mul(self.gas_price)
    }

    /// Calculate remaining gas
    pub fn remaining(&self) -> u64 {
        self.gas_limit.saturating_sub(self.gas_used)
    }

    /// Check if enough gas remains
    pub fn has_enough(&self, gas_units: u64) -> bool {
        self.remaining() >= gas_units
    }

    /// Get gas usage percentage
    pub fn usage_percentage(&self) -> f64 {
        if self.gas_limit == 0 {
            return 0.0;
        }
        (self.gas_used as f64 / self.gas_limit as f64) * 100.0
    }
}

/// Gas estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub gas_units: u64,
    pub gas_price: u64,
    pub total_cost_mist: u64,
    pub total_cost_kanari: f64,
}

impl GasEstimate {
    pub fn new(gas_units: u64, gas_price: u64) -> Self {
        let total_cost_mist = gas_units.saturating_mul(gas_price);
        let total_cost_kanari = total_cost_mist as f64 / 1_000_000_000.0;

        Self {
            gas_units,
            gas_price,
            total_cost_mist,
            total_cost_kanari,
        }
    }

    pub fn from_operation(operation: GasOperation, gas_price: u64) -> Self {
        Self::new(operation.gas_units(), gas_price)
    }
}

/// Gas-related errors
#[derive(Debug, Clone)]
pub enum GasError {
    OutOfGas { required: u64, limit: u64 },
    InsufficientBalance { required: u64, available: u64 },
    PriceTooLow { provided: u64, minimum: u64 },
    Overflow,
}

impl std::fmt::Display for GasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GasError::OutOfGas { required, limit } => {
                write!(
                    f,
                    "Out of gas: required {} but limit is {}",
                    required, limit
                )
            }
            GasError::InsufficientBalance {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient balance for gas: required {} Mist but only {} available",
                    required, available
                )
            }
            GasError::PriceTooLow { provided, minimum } => {
                write!(
                    f,
                    "Gas price too low: provided {} but minimum is {}",
                    provided, minimum
                )
            }
            GasError::Overflow => write!(f, "Gas calculation overflow"),
        }
    }
}

impl std::error::Error for GasError {}

/// Transaction gas info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionGas {
    pub gas_limit: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub gas_refund: u64,
}

impl TransactionGas {
    pub fn new(gas_limit: u64, gas_price: u64) -> Self {
        Self {
            gas_limit,
            gas_price,
            gas_used: 0,
            gas_refund: 0,
        }
    }

    pub fn total_cost(&self) -> u64 {
        self.gas_used.saturating_mul(self.gas_price)
    }

    pub fn refund_amount(&self) -> u64 {
        self.gas_refund.saturating_mul(self.gas_price)
    }

    pub fn net_cost(&self) -> u64 {
        self.total_cost().saturating_sub(self.refund_amount())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_meter_consume() {
        let mut meter = GasMeter::new(100_000, 1000);

        assert!(meter.consume(21_000).is_ok());
        assert_eq!(meter.gas_used, 21_000);
        assert_eq!(meter.remaining(), 79_000);
    }

    #[test]
    fn test_gas_meter_out_of_gas() {
        let mut meter = GasMeter::new(10_000, 1000);

        let result = meter.consume(15_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_gas_operation_costs() {
        assert_eq!(GasOperation::Transfer.gas_units(), 21_000);
        assert_eq!(GasOperation::CreateAccount.gas_units(), 25_000);

        let publish = GasOperation::PublishModule { module_size: 1000 };
        assert_eq!(publish.gas_units(), 60_000); // 50_000 + 1000*10

        let contract_call = GasOperation::ContractCall {
            function_name_len: 10,
        };
        assert_eq!(contract_call.gas_units(), 36_000); // 35_000 + 10*100

        let deployment = GasOperation::ContractDeployment {
            module_size: 1000,
            metadata_size: 200,
        };
        assert_eq!(deployment.gas_units(), 71_000); // 60_000 + 1000*10 + 200*5
    }

    #[test]
    fn test_gas_estimate() {
        let estimate = GasEstimate::new(21_000, 1000);
        assert_eq!(estimate.gas_units, 21_000);
        assert_eq!(estimate.total_cost_mist, 21_000_000);
        assert_eq!(estimate.total_cost_kanari, 0.021);
    }

    #[test]
    fn test_gas_meter_total_cost() {
        let mut meter = GasMeter::new(100_000, 1500);
        meter.consume(21_000).unwrap();

        assert_eq!(meter.total_cost(), 31_500_000); // 21_000 * 1500
    }

    #[test]
    fn test_gas_usage_percentage() {
        let mut meter = GasMeter::new(100_000, 1000);
        meter.consume(25_000).unwrap();

        assert_eq!(meter.usage_percentage(), 25.0);
    }

    #[test]
    fn test_transaction_gas() {
        let mut tx_gas = TransactionGas::new(100_000, 1000);
        tx_gas.gas_used = 21_000;
        tx_gas.gas_refund = 5_000;

        assert_eq!(tx_gas.total_cost(), 21_000_000);
        assert_eq!(tx_gas.refund_amount(), 5_000_000);
        assert_eq!(tx_gas.net_cost(), 16_000_000);
    }
}
