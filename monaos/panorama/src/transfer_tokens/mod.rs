use mona_blockchain::{block::Transaction, blockchain::{get_balance, normalize_address, save_blockchain, BlockchainError, BALANCES}};
use crate::utils::{GAS_FEE_COLLECTOR, calculate_total_transaction_cost};
pub mod verify_transaction;
use verify_transaction::verify_transaction;
use mona_crypto::{load_wallet, sign_message, secure_clear}; // Add mona-crypto imports

/// Transfer tokens from one address to another
pub fn transfer_tokens(
    from_address: &str,
    to_address: &str,
    amount: u64,
    password: &str,
    gas_fee: u64,  // Add gas_fee parameter instead of using constant
) -> Result<Transaction, BlockchainError> {
    // Validate addresses
    if from_address.trim().is_empty() || to_address.trim().is_empty() {
        return Err(BlockchainError::InvalidAddress("Empty address provided".to_string()));
    }
    
    if amount == 0 {
        return Err(BlockchainError::Transaction("Cannot transfer zero tokens".to_string()));
    }
    
    // Parse and normalize addresses
    let from = normalize_address(from_address)?;
    let to = normalize_address(to_address)?;
    
    // Validate addresses are different
    if from == to {
        return Err(BlockchainError::Transaction("Cannot transfer to same address".to_string()));
    }
    
    // Check sender's balance - need to include gas fee
    let balance = get_balance(&from.to_hex_literal())?;
    let total_cost = calculate_total_transaction_cost(amount, gas_fee);
    
    if balance < total_cost {
        return Err(BlockchainError::InsufficientFunds(
            format!("Address {} has {} tokens, tried to send {} + {} gas fee", 
                    from, balance, amount, gas_fee)
        ));
    }
    
    // Parse gas collector address
    let gas_collector = match normalize_address(GAS_FEE_COLLECTOR) {
        Ok(addr) => addr,
        Err(e) => return Err(BlockchainError::Transaction(
            format!("Invalid gas collector address: {}", e)
        )),
    };

    // Create transaction with a unique ID
    let mut transaction = Transaction {
        transaction_id: crate::utils::generate_transaction_id(),
        sender: from,
        receiver: to,
        amount,
        gas_fee, // Store the gas fee in the transaction
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        signature: Vec::new(),
        data: None, // Optional data field for future use
    };
    
    // Sign the transaction with mona-crypto
    let message = transaction.to_signable_message();
    log::debug!("Transaction message to sign (hex): {}", hex::encode(&message));
    
    // Try to load the wallet with mona-crypto
    let address_str = from.to_hex_literal();
    log::debug!("Trying to load wallet for address: {}", address_str);
    
    // Try to sign with mona-crypto
    match load_wallet(&address_str, password) {
        Ok(wallet) => {
            log::debug!("Successfully loaded wallet with curve type: {:?}", wallet.curve_type);
            
            // Sign the transaction with mona-crypto
            match wallet.sign(&message, password) {
                Ok(signature) => {
                    let sig_len = signature.len();
                    log::debug!("Successfully signed transaction: signature length = {}", sig_len);
                    transaction.signature = signature;
                    
                    // Try verification right away for debugging
                    match verify_transaction(&transaction) {
                        Ok(true) => log::debug!("Signature verification successful immediately after signing"),
                        Ok(false) => log::warn!("Signature verification failed immediately after signing"),
                        Err(e) => log::warn!("Error verifying signature: {}", e),
                    }
                },
                Err(e) => {
                    log::error!("Transaction signing failed: {}", e);
                    return Err(BlockchainError::Transaction(format!("Failed to sign: {}", e)));
                }
            }
        },
        Err(e) => {
            log::error!("Failed to load wallet: {}", e);
            return Err(BlockchainError::Transaction(format!("Failed to load wallet: {}", e)));
        }
    }
    
    let mut balances = match BALANCES.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(BlockchainError::Transaction("Failed to lock balances".to_string())),
    };
    
    *balances.entry(from.to_hex_literal()).or_insert(0) -= total_cost;
    *balances.entry(to.to_hex_literal()).or_insert(0) += amount;
    *balances.entry(gas_collector.to_hex_literal()).or_insert(0) += gas_fee;
    
    drop(balances);
    
    if let Err(e) = save_blockchain() {
        log::error!("Failed to save blockchain after transfer: {}", e);
    }
    
    log::info!("Transferred {} tokens from {} to {} with {} gas fee to {}",
              amount, from, to, gas_fee, GAS_FEE_COLLECTOR);
    
    Ok(transaction)
}
