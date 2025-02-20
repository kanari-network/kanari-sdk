use std::collections::BTreeMap;

/// VM execution errors
#[derive(Debug)]
pub enum VMError {
    InsufficientGas(u64),
    InvalidSignature,
    InvalidTransaction(String),
    ExecutionError(String),
    StateError(String),
}

/// Result of transaction execution
#[derive(Debug)]
pub enum TransactionStatus {
    Success {
        gas_used: u64,
    },
    Failed {
        error: VMError,
        gas_used: u64,
    },
}

/// Represents changes to the blockchain state
#[derive(Debug, Default)]
pub struct ChangeSet {
    writes: BTreeMap<Vec<u8>, Vec<u8>>,
    deletes: Vec<Vec<u8>>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.writes.insert(key, value);
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.deletes.push(key);
    }
}

/// Transaction context containing execution parameters
#[derive(Debug)]
pub struct TransactionContext {
    pub max_gas_units: u64,
    pub gas_unit_price: u64,
    pub sender: Vec<u8>,
    pub sequence_number: u64,
}

/// Main VM for executing transactions
pub struct MonaVM {
    state: BTreeMap<Vec<u8>, Vec<u8>>,
    gas_schedule: BTreeMap<String, u64>,
}

impl MonaVM {
    pub fn new() -> Self {
        let mut gas_schedule = BTreeMap::new();
        gas_schedule.insert("execute_instruction".to_string(), 1);
        gas_schedule.insert("write_storage".to_string(), 10);
        gas_schedule.insert("read_storage".to_string(), 5);

        Self {
            state: BTreeMap::new(),
            gas_schedule,
        }
    }

    /// Execute a single transaction with context
    pub fn execute_transaction(
        &mut self,
        transaction: Vec<u8>,
        context: TransactionContext,
    ) -> (TransactionStatus, Option<ChangeSet>) {
        let mut gas_left = context.max_gas_units;

        // 1. Prologue checks
        if let Err(e) = self.run_prologue(&transaction, &context, &mut gas_left) {
            return (
                TransactionStatus::Failed {
                    error: e,
                    gas_used: context.max_gas_units - gas_left,
                },
                None,
            );
        }

        // 2. Main execution
        let (status, mut changes) = match self.execute_payload(&transaction, &mut gas_left) {
            Ok(changes) => (TransactionStatus::Success { gas_used: context.max_gas_units - gas_left }, changes),
            Err(e) => {
                if self.handle_aborted_execution(&transaction) {
                    (
                        TransactionStatus::Success { gas_used: context.max_gas_units - gas_left },
                        ChangeSet::new(),
                    )
                } else {
                    return (
                        TransactionStatus::Failed {
                            error: e,
                            gas_used: context.max_gas_units - gas_left,
                        },
                        None,
                    );
                }
            }
        };

        // 3. Epilogue
        if let Err(e) = self.run_epilogue(&context, &mut changes, &mut gas_left) {
            return (
                TransactionStatus::Failed {
                    error: e,
                    gas_used: context.max_gas_units - gas_left,
                },
                None,
            );
        }

        (status, Some(changes))
    }

    fn run_prologue(
        &self,
        transaction: &[u8],
        context: &TransactionContext,
        gas_left: &mut u64,
    ) -> Result<(), VMError> {
        // Verify transaction signature
        self.verify_signature(transaction)?;

        // Check gas limit
        if context.max_gas_units > self.max_gas_units() {
            return Err(VMError::InvalidTransaction("Gas limit too high".to_string()));
        }

        // Check account balance
        let required_balance = context.max_gas_units * context.gas_unit_price;
        if !self.has_sufficient_balance(&context.sender, required_balance) {
            return Err(VMError::InsufficientGas(required_balance));
        }

        // Deduct gas
        *gas_left = self.deduct_gas(*gas_left, 100)?;
        
        Ok(())
    }

    fn execute_payload(
        &self,
        transaction: &[u8],
        gas_left: &mut u64,
    ) -> Result<ChangeSet, VMError> {
        // Track gas for execution
        *gas_left = self.deduct_gas(*gas_left, self.gas_schedule["execute_instruction"])?;

        // Execute Move code via MoveVM would go here
        Ok(ChangeSet::new())
    }

    fn run_epilogue(
        &self,
        context: &TransactionContext,
        changes: &mut ChangeSet,
        gas_left: &mut u64,
    ) -> Result<(), VMError> {
        let gas_used = context.max_gas_units - *gas_left;
        let gas_charge = gas_used * context.gas_unit_price;

        // Charge gas
        changes.write(
            context.sender.clone(),
            self.get_new_balance(&context.sender, gas_charge)?,
        );

        Ok(())
    }

    // Helper functions
    fn verify_signature(&self, _transaction: &[u8]) -> Result<(), VMError> {
        // Signature verification logic
        Ok(())
    }

    fn max_gas_units(&self) -> u64 {
        1_000_000
    }

    fn has_sufficient_balance(&self, account: &[u8], amount: u64) -> bool {
        self.get_balance(account) >= amount
    }

    fn get_balance(&self, account: &[u8]) -> u64 {
        self.state
            .get(account)
            .map(|bytes| u64::from_le_bytes(bytes.as_slice().try_into().unwrap_or_default()))
            .unwrap_or(0)
    }

    fn get_new_balance(&self, account: &[u8], charge: u64) -> Result<Vec<u8>, VMError> {
        let balance = self.get_balance(account);
        if balance < charge {
            return Err(VMError::InsufficientGas(charge));
        }
        Ok((balance - charge).to_le_bytes().to_vec())
    }

    fn deduct_gas(&self, gas_left: u64, gas_used: u64) -> Result<u64, VMError> {
        gas_left.checked_sub(gas_used).ok_or(VMError::InsufficientGas(gas_used))
    }

    fn handle_aborted_execution(&self, _transaction: &[u8]) -> bool {
        false
    }
}


