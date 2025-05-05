use anyhow::Result;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use move_package::BuildConfig;
use serde_json::json;
use std::path::PathBuf;

use mona_vm::*;
use common::*;

#[derive(Parser)]
#[clap(
    about = "Publish Move modules to blockchain network",
    after_help = "EXAMPLES:
    # Publish module from current directory
    kari move publish

    # Publish module with specific path
    kari move publish path/to/module

    # Specify gas budget
    kari move publish --gas-budget=5000000

    # Publish to specific address
    kari move publish --address=0x123abc

    # Skip verification
    kari move publish --skip-verify"
)]
pub struct Publish {
    /// Path to module directory to be published
    #[clap(long, help = "Directory path containing the Move package to publish")]
    pub module_path: PathBuf,

    /// Gas budget for deployment (default: 3,000,000 units)
    /// Default is 3_000_000 - 10_000_000 for local testing
    #[clap(long, default_value = "3_000_000", help = "Amount of gas units allocated for deployment")]
    pub gas_budget: u64,

    /// Skip verification steps for faster deployment
    #[clap(long, help = "Skip module verification (not recommended for production)")]
    pub skip_verify: bool,
    
    /// Address to publish the module to (uses wallet address or defaults to 0x1 if not specified)
    #[clap(long, help = "Blockchain address to deploy the module to (format: 0x...)")]
    pub address: Option<AccountAddress>,
}

impl Publish {
    pub fn execute(
        self,
        path: Option<PathBuf>,
        config: BuildConfig,
    ) -> Result<()> {
        let package_path = path.unwrap_or_else(|| self.module_path.clone());
        
        // Set default address if none provided - first try from config, then fallback to 0x1
        let address = self.address.unwrap_or_else(|| {
            if let Some(wallet) = get_main_wallet() {
                AccountAddress::from_hex_literal(&format!("0x{}", wallet))
                    .or_else(|_| AccountAddress::from_hex(&wallet))
                    .unwrap_or_else(|_| AccountAddress::from_hex_literal("0x1").unwrap())
            } else {
                AccountAddress::from_hex_literal("0x1").unwrap()
            }
        });

        // Update build config with address
        let mut build_config = config;
        build_config.additional_named_addresses.insert(
            "module_addr".to_string(),
            address
        );

        // Always use the Mona VM for publishing to blockchain
        println!("Publishing with Mona VM to blockchain network...");
        println!("📦 Package path: {}", package_path.display());
        println!("🔑 Target address: 0x{}", address.to_hex());
        println!("⛽ Gas budget: {}", self.gas_budget);
        
        // Create timer to measure deployment duration
        let start_time = std::time::Instant::now();
        
        // Create Mona VM publish instance
        let mona_vm_publish = mona_vm::Publish;
        
        // Execute the deployment
        match mona_vm_publish.execute(
            Some(package_path.clone()),
            Some(address),
            build_config,
            Some(self.gas_budget),
            self.skip_verify
        ) {
            Ok(()) => {
                // Calculate elapsed time
                let duration = start_time.elapsed();
                
                // Try to extract VM state information to display deployed modules
                if let Ok(vm_state) = VM_STATE.read() {
                    let modules = vm_state.modules.values()
                        .filter(|m| m.address == address)
                        .collect::<Vec<_>>();
                    
                    if !modules.is_empty() {
                        println!("\n✅ Successfully deployed {} modules to blockchain:", modules.len());
                        for module in modules {
                            println!("  • Module: {}", module.name);
                            println!("    Module ID: {}", module.module_id);
                            println!("    Size: {} bytes", module.bytecode.len());
                            println!("    Public functions: {}", module.public_functions.join(", "));
                        }
                    }
                }
                
                println!("\n✅ Deployment completed in {:.2?}", duration);
                
                // Create a structured result
                let result = json!({
                    "status": "success",
                    "address": format!("0x{}", address.to_hex()),
                    "deployment_time_ms": duration.as_millis(),
                    "gas_budget": self.gas_budget
                });
                
                println!("\nDeployment Result: {}", serde_json::to_string_pretty(&result)?);
                Ok(())
            },
            Err(e) => {
                // Calculate elapsed time even for failed deployments
                let duration = start_time.elapsed();
                
                // Create a structured error result
                let error_result = json!({
                    "status": "error",
                    "type": "blockchain_deployment",
                    "message": e.to_string(),
                    "details": {
                        "package_path": package_path.to_string_lossy(),
                        "address": format!("0x{}", address.to_hex()),
                        "gas_budget": self.gas_budget,
                        "skip_verification": self.skip_verify,
                        "elapsed_time_ms": duration.as_millis()
                    }
                });
                
                // Print the error in a structured way
                println!("\n❌ Deployment failed after {:.2?}", duration);
                println!("Error: {}\n", e);
                println!("Error Details: {}", serde_json::to_string_pretty(&error_result)?);
                
                // Return the original error
                Err(anyhow::anyhow!("Mona VM blockchain deployment failed: {}", e))
            }
        }
    }
}
