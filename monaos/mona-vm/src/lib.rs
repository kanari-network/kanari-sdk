use mona_types::address::Address;
use mona_types::gas::{GasError, GasMeter, GasSchedule};
use std::collections::BTreeMap;
use move_core_types::language_storage::TypeTag;


/// VM execution errors with improved error handling
#[derive(Debug)]
pub enum VMError {
    InsufficientGas { required: u64, available: u64 },
    InvalidSignature,
    InvalidTransaction(String),
    ExecutionError(String),
    StateError(String),
    GasError(GasError),
}

/// Result of transaction execution with detailed status
#[derive(Debug)]
pub enum TransactionStatus {
    Success { gas_used: u64, changes: ChangeSet },
    Failed { error: VMError, gas_used: u64 },
}

/// State changes with improved storage operations
#[derive(Debug, Default)]
pub struct ChangeSet {
    writes: BTreeMap<Vec<u8>, Vec<u8>>,
    deletes: Vec<Vec<u8>>,
    gas_used: u64,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&mut self, key: Vec<u8>, value: Vec<u8>) -> &mut Self {
        self.writes.insert(key, value);
        self
    }

    pub fn delete(&mut self, key: Vec<u8>) -> &mut Self {
        self.deletes.push(key);
        self
    }

    pub fn record_gas(&mut self, amount: u64) -> &mut Self {
        self.gas_used += amount;
        self
    }
}

/// Enhanced transaction context
#[derive(Debug)]
pub struct TransactionContext {
    pub max_gas_units: u64,
    pub gas_unit_price: u64,
    pub sender: Address,
    pub sequence_number: u64,
    pub expiration_timestamp_secs: u64,
}

/// Main VM implementation
pub struct MonaVM {
    state: BTreeMap<Vec<u8>, Vec<u8>>,
    gas_schedule: GasSchedule,
}

impl MonaVM {
    pub fn new() -> Self {
        Self {
            state: BTreeMap::new(),
            gas_schedule: GasSchedule::default(),
        }
    }

    pub fn with_gas_schedule(gas_schedule: GasSchedule) -> Self {
        Self {
            state: BTreeMap::new(),
            gas_schedule,
        }
    }

    /// Execute a transaction with improved error handling and gas tracking
    pub fn execute_transaction(
        &mut self,
        transaction: Vec<u8>,
        context: TransactionContext,
    ) -> TransactionStatus {
        let mut gas_meter = GasMeter::new(context.max_gas_units, self.gas_schedule.clone());
        let mut changes = ChangeSet::new();

        match self.execute_inner(&transaction, &context, &mut gas_meter, &mut changes) {
            Ok(()) => TransactionStatus::Success {
                gas_used: context.max_gas_units - gas_meter.gas_left(),
                changes,
            },
            Err(error) => TransactionStatus::Failed {
                error,
                gas_used: context.max_gas_units - gas_meter.gas_left(),
            },
        }
    }

    fn execute_inner(
        &self,
        transaction: &[u8],
        context: &TransactionContext,
        gas_meter: &mut GasMeter,
        changes: &mut ChangeSet,
    ) -> Result<(), VMError> {
        // 1. Prologue checks
        self.run_prologue(transaction, context, gas_meter)?;

        // 2. Execute main transaction logic
        self.execute_payload(transaction, gas_meter, changes)?;

        // 3. Run epilogue
        self.run_epilogue(context, changes, gas_meter)?;

        Ok(())
    }

    fn run_prologue(
        &self,
        transaction: &[u8],
        context: &TransactionContext,
        gas_meter: &mut GasMeter,
    ) -> Result<(), VMError> {
        // Verify transaction
        self.verify_signature(transaction)?;
        self.verify_transaction(context)?;

        // Charge initial gas cost
        gas_meter
            .charge_storage_op(transaction.len(), false)
            .map_err(|e| VMError::GasError(e))?;

        Ok(())
    }

    fn execute_payload(
        &self,
        transaction: &[u8],
        gas_meter: &mut GasMeter,
        changes: &mut ChangeSet,
    ) -> Result<(), VMError> {
        // Execute transaction instructions
        let instruction_gas = self.calculate_instruction_gas(transaction);
        gas_meter
            .deduct_gas(instruction_gas)
            .map_err(|e| VMError::GasError(e))?;

        // Record gas usage in changes
        changes.record_gas(instruction_gas);

        Ok(())
    }

    fn run_epilogue(
        &self,
        context: &TransactionContext,
        changes: &mut ChangeSet,
        gas_meter: &mut GasMeter,
    ) -> Result<(), VMError> {
        let gas_used = context.max_gas_units - gas_meter.gas_left();
        let gas_charge = gas_used * context.gas_unit_price;

        let new_balance = self.calculate_new_balance(&context.sender, gas_charge)?;
        changes.write(context.sender.to_bytes().to_vec(), new_balance);

        Ok(())
    }

    // Helper functions
    fn verify_signature(&self, transaction: &[u8]) -> Result<(), VMError> {
        // Implement signature verification
        Ok(())
    }

    fn verify_transaction(&self, context: &TransactionContext) -> Result<(), VMError> {
        // Use the correct field from GasSchedule
        if context.max_gas_units > self.gas_schedule.custom_costs
            .get("max_gas_per_tx")
            .copied()
            .unwrap_or(1_000_000) 
        {
            return Err(VMError::InvalidTransaction("Gas limit too high".to_string()));
        }
        Ok(())
    }

    fn calculate_instruction_gas(&self, transaction: &[u8]) -> u64 {
        self.gas_schedule.instruction_cost * transaction.len() as u64
    }

    fn calculate_new_balance(&self, account: &Address, charge: u64) -> Result<Vec<u8>, VMError> {
        let current = self.get_balance(account);
        current
            .checked_sub(charge)
            .ok_or_else(|| VMError::InsufficientGas {
                required: charge,
                available: current,
            })
            .map(|balance| balance.to_le_bytes().to_vec())
    }

    fn get_balance(&self, account: &Address) -> u64 {
        self.state
            .get(&account.to_bytes().to_vec())  // Convert address bytes to Vec<u8> for BTreeMap key
            .and_then(|bytes| bytes.get(..8))
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_context() -> TransactionContext {
        TransactionContext {
            max_gas_units: 1000,
            gas_unit_price: 1,
            sender: Address::new([0; 32]),
            sequence_number: 0,
            expiration_timestamp_secs: 0,
        }
    }

    #[test]
    fn test_successful_transaction() {
        let mut vm = MonaVM::new();
        let context = setup_test_context();
        let transaction = vec![1, 2, 3];

        match vm.execute_transaction(transaction, context) {
            TransactionStatus::Success { gas_used, changes } => {
                assert!(gas_used > 0);
                assert!(gas_used < 1000);
                assert_eq!(changes.gas_used, gas_used);
            }
            TransactionStatus::Failed { .. } => panic!("Expected successful transaction"),
        }
    }

    #[test]
    fn test_out_of_gas_transaction() {
        let mut vm = MonaVM::new();
        let mut context = setup_test_context();
        context.max_gas_units = 1;
        let transaction = vec![1, 2, 3];

        match vm.execute_transaction(transaction, context) {
            TransactionStatus::Failed {
                error: VMError::GasError(_),
                gas_used,
            } => {
                assert_eq!(gas_used, 1);
            }
            _ => panic!("Expected out of gas error"),
        }
    }
}
