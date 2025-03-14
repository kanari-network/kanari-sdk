use log::error;
use crate::transaction::Transaction;
use mona_types::address::Address;
use mona_vm::{TransactionContext, TransactionStatus};
use super::{get_mutex_lock, SimulationError, VM};

// Helper function to process a transaction in the VM
pub fn process_transaction_in_vm(transaction: &Transaction) -> Result<TransactionStatus, SimulationError> {
    let mut vm = get_mutex_lock(&VM, "process_transaction_vm")?;
    
    // Convert transaction to VM format safely
    let tx_bytes = transaction.serialize();
    let sender_address = Address::from_hex(&transaction.sender)
        .map_err(|e| {
            error!("Failed to convert sender address: {} for tx: {}", e, transaction.hash);
            SimulationError::AddressError(e.to_string())
        })?;

    // Create transaction context
    let context = TransactionContext {
        max_gas_units: transaction.gas_limit,
        gas_unit_price: transaction.gas_price,
        sender: sender_address,
        sequence_number: transaction.nonce,
        expiration_timestamp_secs: transaction.timestamp,
    };

    // Execute transaction in VM
    Ok(vm.execute_transaction(tx_bytes, context))
}