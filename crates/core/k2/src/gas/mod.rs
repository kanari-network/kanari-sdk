

/// Trait for gas calculation operations
pub trait GasCalculator {
    fn calculate_gas(&self) -> f64;
}

/// Gas calculation implementation
pub struct TransactionGas;

pub enum GasTransaction {
    Basic(BasicTransaction),
    SmartContract(SmartContractTransaction),
    MintNFT(MintNFTTransaction),
    TransferNFT(TransferNFTTransaction),
    Payment(PaymentTransaction),
    MoveModuleDeploy(MoveModuleDeployTransaction),
    MoveFunctionCall(MoveFunctionCallTransact),
}

// Transaction type definitions
pub struct BasicTransaction;
pub struct SmartContractTransaction;
pub struct MintNFTTransaction;
pub struct TransferNFTTransaction;
pub struct PaymentTransaction;
pub struct MoveModuleDeployTransaction;
pub struct MoveFunctionCallTransact;

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
            let density_factor = self.transactions_per_block as f64 / BASE_TXS_PER_BLOCK as f64;
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


impl GasCalculator for BasicTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::basic_transaction(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for SmartContractTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::deploy_smart_contract(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for MintNFTTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::mint_nft(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for TransferNFTTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::transfer_nft(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for PaymentTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::payment(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for MoveModuleDeployTransaction {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::move_module_deploy(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}

impl GasCalculator for MoveFunctionCallTransact {
    fn calculate_gas(&self) -> f64 {
        TransactionGas::with_density(
            TransactionGas::move_function_call(),
            &TransactionDensity::new(get_current_block_transaction_count())
        )
    }
}
impl GasTransaction {
    pub fn calculate_gas(&self) -> f64 {
        match self {
            GasTransaction::Basic(tx) => tx.calculate_gas(),
            GasTransaction::SmartContract(tx) => tx.calculate_gas(),
            GasTransaction::MintNFT(tx) => tx.calculate_gas(),
            GasTransaction::TransferNFT(tx) => tx.calculate_gas(),
            GasTransaction::Payment(tx) => tx.calculate_gas(),
            GasTransaction::MoveModuleDeploy(tx) => tx.calculate_gas(),
            GasTransaction::MoveFunctionCall(tx) => tx.calculate_gas(),
        }
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

pub const MINT_NFT_TRANSACTION_GAS_COST: f64 = 0.00010000;
pub const TRANSFER_NFT_TRANSACTION_GAS_COST: f64 = 0.00000100;
pub const PAYMENT_TRANSACTION_GAS_COST: f64 = 0.00020000;
pub const MOVE_MODULE_DEPLOY_GAS: f64 = 0.00050000;
pub const MOVE_FUNCTION_CALL_GAS: f64 = 0.000154000;
