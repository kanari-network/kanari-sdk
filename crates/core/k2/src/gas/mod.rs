use crate::transaction::Transaction;
use std::sync::mpsc::Sender;

/// Trait for gas calculation operations
pub trait GasCalculator {
    fn calculate_gas(&self) -> f64;
}

/// Gas calculation implementation
pub struct TransactionGas;


/// Transaction density metrics
pub struct TransactionDensity {
    transactions_per_block: u32,
    base_density: f64,
}

impl TransactionDensity {
    pub fn new(transactions_per_block: u32) -> Self {
        Self {
            transactions_per_block,
            base_density: 1.0,
        }
    }

    pub fn calculate_density_multiplier(&self) -> f64 {
        const BASE_TXS_PER_BLOCK: u32 = 100;
        const MAX_MULTIPLIER: f64 = 2.0;
        
        if self.transactions_per_block <= BASE_TXS_PER_BLOCK {
            1.0
        } else {
            let density_factor = (self.transactions_per_block as f64 / BASE_TXS_PER_BLOCK as f64);
            (density_factor * self.base_density).min(MAX_MULTIPLIER)
        }
    }
}


impl TransactionGas {
    /// Calculate basic transaction gas
    pub fn basic_transaction() -> f64 {
        TRANSACTION_GAS_COST
    }

    /// Calculate smart contract deployment gas
    pub fn deploy_smart_contract() -> f64 {
        DEPLOY_SM_TRANSACTION_GAS_COST
    }

    /// Calculate NFT minting gas
    pub fn mint_nft() -> f64 {
        MINT_NFT_TRANSACTION_GAS_COST
    }

    /// Calculate NFT transfer gas
    pub fn transfer_nft() -> f64 {
        TRANSFER_NFT_TRANSACTION_GAS_COST
    }

    /// Calculate payment transaction gas
    pub fn payment() -> f64 {
        PAYMENT_TRANSACTION_GAS_COST
    }

    /// Calculate Move module deployment gas
    pub fn move_module_deploy() -> f64 {
        MOVE_MODULE_DEPLOY_GAS
    }

    /// Calculate Move function call gas
    pub fn move_function_call() -> f64 {
        MOVE_FUNCTION_CALL_GAS
    }

    /// Calculate gas with density adjustment
    pub fn with_density(base_gas: f64, density: &TransactionDensity) -> f64 {
        base_gas * density.calculate_density_multiplier()
    }
}

impl GasCalculator for Transaction {
    fn calculate_gas(&self) -> f64 {
        let density = TransactionDensity::new(get_current_block_transaction_count());
        let base_gas = match self {
            Transaction::Basic(_) => TransactionGas::basic_transaction(),
            Transaction::SmartContract(_) => TransactionGas::deploy_smart_contract(),
            Transaction::MintNFT(_) => TransactionGas::mint_nft(),
            Transaction::TransferNFT(_) => TransactionGas::transfer_nft(),
            Transaction::Payment(_) => TransactionGas::payment(),
            Transaction::MoveModuleDeploy(_) => TransactionGas::move_module_deploy(),
            Transaction::MoveFunctionCall(_) => TransactionGas::move_function_call(),
            _ => TRANSACTION_GAS_COST
        };
        TransactionGas::with_density(base_gas, &density)
    }
}

// Add this function to get current block transaction count
// This should be implemented based on your blockchain implementation
fn get_current_block_transaction_count() -> u32 {
    // Placeholder - implement actual block transaction count retrieval
    100
}

// Constants remain unchanged
pub const TRANSACTION_GAS_COST: f64 = 0.00000150;
pub const DEPLOY_SM_TRANSACTION_GAS_COST: f64 = 0.00005000;
pub static mut TRANSACTION_SENDER: Option<Sender<Transaction>> = None;
pub const MINT_NFT_TRANSACTION_GAS_COST: f64 = 0.00010000;
pub const TRANSFER_NFT_TRANSACTION_GAS_COST: f64 = 0.00000100;
pub const PAYMENT_TRANSACTION_GAS_COST: f64 = 0.00020000;
pub const MOVE_MODULE_DEPLOY_GAS: f64 = 0.00050000;
pub const MOVE_FUNCTION_CALL_GAS: f64 = 0.000154000;
