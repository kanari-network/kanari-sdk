use std::path::PathBuf;

use crate::{ChangeSet, PackageValidator, TransactionStatus, VMError};

// Example VM extension for package deployment
pub struct VM {
    // existing fields...
    package_validator: PackageValidator,
}

impl VM {
    pub fn new(framework_path: impl Into<PathBuf>) -> Self {
        // Initialize other fields...
        Self {
            // other fields...
            package_validator: PackageValidator::new(framework_path),
        }
    }
    
    // Example method for deploying a package
    pub fn deploy_package(&self, package_bytes: &[u8], gas_limit: u64) -> TransactionStatus {
        let mut gas_used = 0;
        
        // 1. First verify the package is valid according to framework rules
        let package_info = match self.package_validator.validate_package(package_bytes) {
            Ok(info) => info,
            Err(error) => {
                return TransactionStatus::Failed { 
                    error, 
                    gas_used: gas_used 
                };
            }
        };
        
        // 2. Calculate gas for deployment
        let deployment_gas = self.calculate_deployment_gas(package_bytes);
        gas_used += deployment_gas;
        
        if gas_used > gas_limit {
            return TransactionStatus::Failed { 
                error: VMError::InsufficientGas { 
                    required: gas_used, 
                    available: gas_limit 
                }, 
                gas_used 
            };
        }
        
        // 3. Generate package ID (could be hash of content)
        let package_id = generate_package_id(package_bytes);
        
        // 4. Store package in state
        let mut changes = ChangeSet::new();
        changes
            .write(package_id.clone(), package_bytes.to_vec())
            .add_package(package_id, package_info)
            .record_gas(gas_used);
            
        TransactionStatus::Success { 
            gas_used, 
            changes 
        }
    }
    
    fn calculate_deployment_gas(&self, package_bytes: &[u8]) -> u64 {
        // Simple example: base cost + per byte cost
        let base_cost = 1000;
        let per_byte_cost = 1;
        
        base_cost + (package_bytes.len() as u64 * per_byte_cost)
    }
}

// Helper function to generate package ID
fn generate_package_id(package_bytes: &[u8]) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(package_bytes);
    let result = hasher.finalize();
    result.to_vec()
}