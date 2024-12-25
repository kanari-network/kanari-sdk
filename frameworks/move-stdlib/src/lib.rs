use move_vm_types::*;

/// Gas price constants
pub const GAS_BASE_COST: u64 = 1;
pub const GAS_PER_BYTE: u64 = 1;

/// Gas calculation structure
pub struct GasCalculator {
    remaining_gas: u64,
    gas_used: u64,
}

impl GasCalculator {
    pub fn new(gas_limit: u64) -> Self {
        Self {
            remaining_gas: gas_limit,
            gas_used: 0,
        }
    }

    pub fn consume_gas(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.remaining_gas < amount {
            return Err("Insufficient gas");
        }
        self.remaining_gas -= amount;
        self.gas_used += amount;
        Ok(())
    }

    pub fn calculate_execution_gas(&self, instruction_count: u64) -> u64 {
        GAS_BASE_COST + (instruction_count * GAS_PER_BYTE)
    }

    pub fn get_remaining_gas(&self) -> u64 {
        self.remaining_gas
    }

    pub fn get_gas_used(&self) -> u64 {
        self.gas_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_calculation() {
        let mut calc = GasCalculator::new(100);
        assert_eq!(calc.get_remaining_gas(), 100);
        assert!(calc.consume_gas(50).is_ok());
        assert_eq!(calc.get_remaining_gas(), 50);
        assert_eq!(calc.get_gas_used(), 50);
    }
}