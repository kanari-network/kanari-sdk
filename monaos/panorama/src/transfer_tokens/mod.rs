use crate::{block::Transaction, blockchain::{get_balance, normalize_address, save_blockchain, BlockchainError, BALANCES}};
use crate::utils::{GAS_FEE_COLLECTOR, calculate_total_transaction_cost};
pub mod verify_transaction;
use verify_transaction::verify_transaction;

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
        signature: Vec::new(), // Initialize with empty signature
    };
    
    // Sign the transaction with better debugging
    let message = transaction.to_signable_message();
    log::debug!("Transaction message to sign (hex): {}", hex::encode(&message));
    
    // Try to load the wallet to determine the curve type
    let address_str = from.to_hex_literal();
    log::debug!("Trying to load wallet for address: {}", address_str);
    
    // Try to load the wallet directly (this way we can get the private key)
    match key::load_wallet(&address_str, password) {
        Ok(wallet) => {
            log::debug!("Successfully loaded wallet with curve type: {:?}", wallet.curve_type);
            
            // We need to use strings without the 0x prefix for signing
            let _clean_address = address_str.trim_start_matches("0x");
            
            // Directly sign with the correct private key and curve type
            let signature_result = match wallet.curve_type {
                key::CurveType::K256 => {
                    log::debug!("Using K256 signing with private key");
                    key::sign_message_k256(&wallet.private_key, &message)
                },
                key::CurveType::P256 => {
                    log::debug!("Using P256 signing with private key");
                    key::sign_message_p256(&wallet.private_key, &message)
                },
            };
            
            match signature_result {
                Ok(signature) => {
                    let sig_len = signature.len();
                    log::debug!("Successfully signed transaction: signature length = {}", sig_len);
                    transaction.signature = signature;
                    
                    // Try verification right away for debugging
                    match transaction.verify_signature() {
                        Ok(true) => log::debug!("Signature verification successful immediately after signing"),
                        Ok(false) => log::warn!("Signature verification failed immediately after signing"),
                        Err(e) => log::warn!("Error verifying signature: {}", e),
                    }
                },
                Err(e) => {
                    log::error!("Direct signing failed: {}", e);
                    return Err(BlockchainError::Transaction(format!("Failed to sign: {}", e)));
                }
            }
        },
        Err(e) => {
            // Fall back to wallet-less signing (this is less reliable)
            log::warn!("Failed to load wallet: {}, trying without private key", e);
            
            match key::sign_message(&key::Wallet { 
                address: from, 
                private_key: String::new(), // Temporary, will be loaded from wallet
                seed_phrase: String::new(), 
                curve_type: key::CurveType::K256, // Default, will be determined by wallet
            }, &message, password) {
                Ok(signature) => {
                    transaction.signature = signature;
                },
                Err(_e) => {
                    let mut tried_p256 = false;
                    let mut tried_k256 = false;
                    
                    if !tried_k256 {
                        tried_k256 = true;
                        log::debug!("Trying K256 signing");
                        match key::sign_message(&key::Wallet { 
                            address: from, 
                            private_key: String::new(),
                            seed_phrase: String::new(), 
                            curve_type: key::CurveType::K256,
                        }, &message, password) {
                            Ok(signature) => {
                                transaction.signature = signature;
                                log::debug!("K256 signing succeeded");
                            },
                            Err(k256_err) => {
                                log::debug!("K256 signing failed: {}", k256_err);
                                if !tried_p256 {
                                    tried_p256 = true;
                                    log::debug!("Trying P256 signing");
                                    match key::sign_message(&key::Wallet { 
                                        address: from, 
                                        private_key: String::new(),
                                        seed_phrase: String::new(), 
                                        curve_type: key::CurveType::P256,
                                    }, &message, password) {
                                        Ok(signature) => {
                                            transaction.signature = signature;
                                            log::debug!("P256 signing succeeded");
                                        },
                                        Err(p256_err) => {
                                            log::error!("Both K256 and P256 signing failed: K256 error: {}, P256 error: {}", 
                                                      k256_err, p256_err);
                                            return Err(BlockchainError::Transaction(
                                                format!("Failed to sign transaction with both curve types.")
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if !tried_p256 && transaction.signature.is_empty() {
                        tried_p256 = true;
                        log::debug!("Trying P256 signing");
                        match key::sign_message(&key::Wallet { 
                            address: from, 
                            private_key: String::new(),
                            seed_phrase: String::new(), 
                            curve_type: key::CurveType::P256,
                        }, &message, password) {
                            Ok(signature) => {
                                transaction.signature = signature;
                                log::debug!("P256 signing succeeded");
                            },
                            Err(p256_err) => {
                                log::error!("P256 signing failed: {}", p256_err);
                                if !tried_k256 {
                                    log::debug!("Trying K256 as last resort");
                                    match key::sign_message(&key::Wallet { 
                                        address: from, 
                                        private_key: String::new(),
                                        seed_phrase: String::new(), 
                                        curve_type: key::CurveType::K256,
                                    }, &message, password) {
                                        Ok(signature) => {
                                            transaction.signature = signature;
                                            log::debug!("K256 signing succeeded");
                                        },
                                        Err(k256_err) => {
                                            log::error!("Both curve types failed: P256 error: {}, K256 error: {}", 
                                                       p256_err, k256_err);
                                            return Err(BlockchainError::Transaction(
                                                format!("Failed to sign transaction with both curve types.")
                                            ));
                                        }
                                    }
                                } else {
                                    return Err(BlockchainError::Transaction(
                                        format!("Failed to sign transaction with both curve types.")
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if !transaction.signature.is_empty() {
        match verify_transaction(&transaction) {
            Ok(true) => log::debug!("Transaction signature verified successfully"),
            Ok(false) => log::warn!("Transaction signature verification failed"),
            Err(e) => log::warn!("Error verifying transaction signature: {}", e),
        }
    } else {
        log::warn!("Transaction was not signed - signature is empty");
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
