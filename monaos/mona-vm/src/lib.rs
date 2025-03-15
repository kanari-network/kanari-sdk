use mona_types::address::Address;
use mona_types::gas::{GasError, GasMeter, GasSchedule};
use std::collections::BTreeMap;

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

    // Add these accessor methods
    pub fn get_writes(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.writes
    }

    pub fn get_deletes(&self) -> &Vec<Vec<u8>> {
        &self.deletes
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
        self.execute_payload(transaction, context, gas_meter, changes)?;

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
        context: &TransactionContext,
        gas_meter: &mut GasMeter,
        changes: &mut ChangeSet,
    ) -> Result<(), VMError> {
        if transaction.is_empty() {
            return Err(VMError::ExecutionError("Empty transaction payload".to_string()));
        }
    
        let action_code = transaction[0];
    
        match action_code {
            1 => { // Coin Transfer Action
                if transaction.len() < 1 + 32 + 8 {
                    return Err(VMError::InvalidTransaction("Transfer transaction payload too short".to_string()));
                }
    
                let recipient_address_bytes = &transaction[1..33];
                let amount_bytes = &transaction[33..41];
    
                // Create recipient address from bytes
                let mut address_bytes = [0u8; Address::LENGTH];
                address_bytes.copy_from_slice(recipient_address_bytes);
                let recipient = Address::new(address_bytes);
                
                // Parse amount as u64 from bytes
                let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
                
                // Execute the transfer with proper context
                self.execute_transfer(&recipient, amount, context, changes, gas_meter)?;
            },
            2 => { // Multi-coin Transfer (supports different coin types)
                if transaction.len() < 1 + 32 + 8 + 4 {
                    return Err(VMError::InvalidTransaction("Multi-coin transfer payload too short".to_string()));
                }
                
                let recipient_address_bytes = &transaction[1..33];
                let amount_bytes = &transaction[33..41];
                let coin_type_len_bytes = &transaction[41..45];
                
                // Parse coin type length
                let coin_type_len = u32::from_le_bytes(coin_type_len_bytes.try_into().unwrap()) as usize;
                
                if transaction.len() < 1 + 32 + 8 + 4 + coin_type_len {
                    return Err(VMError::InvalidTransaction("Coin type data incomplete".to_string()));
                }
                
                // Extract coin type string
                let coin_type_bytes = &transaction[45..(45 + coin_type_len)];
                let coin_type = match std::str::from_utf8(coin_type_bytes) {
                    Ok(s) => s.to_string(),
                    Err(_) => return Err(VMError::InvalidTransaction("Invalid coin type encoding".to_string()))
                };
                
                // Create recipient address
                let mut address_bytes = [0u8; Address::LENGTH];
                address_bytes.copy_from_slice(recipient_address_bytes);
                let recipient = Address::new(address_bytes);
                
                // Parse amount
                let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
                
                // Execute multi-coin transfer
                self.execute_multi_coin_transfer(&recipient, amount, &coin_type, context, changes, gas_meter)?;
            },
            _ => {
                // Execute other transaction instructions
                let instruction_gas = self.calculate_instruction_gas(transaction);
                gas_meter
                    .deduct_gas(instruction_gas)
                    .map_err(|e| VMError::GasError(e))?;
                changes.record_gas(instruction_gas);
            }
        }
    
        Ok(())
    }

    fn execute_transfer(
        &self,
        recipient: &Address,
        amount: u64,
        context: &TransactionContext,
        changes: &mut ChangeSet,
        gas_meter: &mut GasMeter,
    ) -> Result<(), VMError> {
        // Use write_cost as the gas fee for transfer operations
        let transfer_gas = self.gas_schedule.write_cost;
        
        // Charge gas for the transfer operation
        gas_meter
            .charge_storage_op(transfer_gas as usize, false)
            .map_err(|e| VMError::GasError(e))?;
        changes.record_gas(transfer_gas);
    
        // Check if sender has sufficient balance
        let sender_balance = self.get_balance(&context.sender);
        if sender_balance < amount {
            return Err(VMError::InsufficientGas {
                required: amount,
                available: sender_balance,
            });
        }
    
        // Debit the sender's account
        let new_sender_balance = sender_balance - amount;
        changes.write(context.sender.to_bytes().to_vec(), new_sender_balance.to_le_bytes().to_vec());
    
        // Credit the recipient's account
        let recipient_balance = self.get_balance(recipient);
        let new_recipient_balance = recipient_balance + amount;
        changes.write(recipient.to_bytes().to_vec(), new_recipient_balance.to_le_bytes().to_vec());
    
        Ok(())
    }

    // New function to handle multi-coin transfers
    fn execute_multi_coin_transfer(
        &self,
        recipient: &Address,
        amount: u64,
        coin_type: &str,
        context: &TransactionContext,
        changes: &mut ChangeSet,
        gas_meter: &mut GasMeter,
    ) -> Result<(), VMError> {
        // Higher gas cost for multi-coin transfers
        let transfer_gas = self.gas_schedule.write_cost * 2;
        
        // Charge gas for the transfer operation
        gas_meter
            .charge_storage_op(transfer_gas as usize, false)
            .map_err(|e| VMError::GasError(e))?;
        changes.record_gas(transfer_gas);
        
        // Generate keys with coin type suffix
        let sender_key = self.get_coin_balance_key(&context.sender, coin_type);
        let recipient_key = self.get_coin_balance_key(recipient, coin_type);
        
        // Check if sender has sufficient balance for this coin type
        let sender_balance = self.get_coin_balance(&context.sender, coin_type);
        if sender_balance < amount {
            return Err(VMError::InsufficientGas {
                required: amount,
                available: sender_balance,
            });
        }
        
        // Debit the sender
        let new_sender_balance = sender_balance - amount;
        changes.write(sender_key, new_sender_balance.to_le_bytes().to_vec());
        
        // Credit the recipient
        let recipient_balance = self.get_coin_balance(recipient, coin_type);
        let new_recipient_balance = recipient_balance + amount;
        changes.write(recipient_key, new_recipient_balance.to_le_bytes().to_vec());
        
        Ok(())
    }
    
    // Helper method to get balance for a specific coin type
    fn get_coin_balance(&self, account: &Address, coin_type: &str) -> u64 {
        let key = self.get_coin_balance_key(account, coin_type);
        
        self.state
            .get(&key)
            .and_then(|bytes| bytes.get(..8))
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0)
    }
    
    // Helper method to generate a key for coin-specific balances
    fn get_coin_balance_key(&self, account: &Address, coin_type: &str) -> Vec<u8> {
        let mut key = account.to_bytes().to_vec();
        key.extend_from_slice(b":");
        key.extend_from_slice(coin_type.as_bytes());
        key
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

    fn verify_signature(&self, transaction: &[u8]) -> Result<(), VMError> {
        // Implement proper signature verification
        // This is a simple example - you should use proper signature verification
        if transaction.is_empty() {
            return Err(VMError::InvalidSignature);
        }
        Ok(())
    }

    fn verify_transaction(&self, context: &TransactionContext) -> Result<(), VMError> {
        // Use the correct field from GasSchedule
        if context.max_gas_units
            > self
                .gas_schedule
                .custom_costs
                .get("max_gas_per_tx")
                .copied()
                .unwrap_or(1_000_000)
        {
            return Err(VMError::InvalidTransaction(
                "Gas limit too high".to_string(),
            ));
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
            .get(&account.to_bytes().to_vec()) // Convert address bytes to Vec<u8> for BTreeMap key
            .and_then(|bytes| bytes.get(..8))
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0)
    }
}