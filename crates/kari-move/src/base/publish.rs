use anyhow::Result;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use move_package::BuildConfig;
use serde_json::json;
use std::path::PathBuf;

use mona_vm::*;
use common::*;
use mona_crypto::{load_wallet, WalletError};
use sha3::{Digest, Sha3_256};

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
    kari move publish --skip-verify

    # Use wallet password
    kari move publish --password=your_password"
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

    /// Password for wallet to sign transaction
    #[clap(long, help = "Password for wallet to sign deployment transaction")]
    pub password: Option<String>,
}

impl Publish {
    pub fn execute(
        self,
        path: Option<PathBuf>,
        config: BuildConfig,
    ) -> Result<()> {
        let package_path = path.unwrap_or_else(|| self.module_path.clone());
        
        // Check if package path exists
        if !package_path.exists() {
            return Err(anyhow::anyhow!("Package path does not exist: {}", package_path.display()));
        }
        println!("📋 Found package directory at: {}", package_path.display());
        
        // Check if sources directory exists
        let sources_dir = package_path.join("sources");
        if !sources_dir.exists() {
            return Err(anyhow::anyhow!("No 'sources' directory found at path: {}", package_path.display()));
        }
        println!("📋 Found sources directory with Move files");
        
        // Set default address if none provided - first try from config, then fallback to 0x1
        let address = self.address.unwrap_or_else(|| {
            if let Some(wallet) = get_main_wallet() {
                // Fix: Check if wallet already has 0x prefix before adding it
                let wallet_with_prefix = if wallet.starts_with("0x") {
                    wallet.clone()
                } else {
                    format!("0x{}", wallet)
                };
                
                AccountAddress::from_hex_literal(&wallet_with_prefix)
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
        
        // สร้าง transaction payload ที่จะถูกลงนาม
        let mut hasher = Sha3_256::new();
        hasher.update(address.to_hex().as_bytes());
        hasher.update(package_path.to_str().unwrap_or("").as_bytes());
        hasher.update(self.gas_budget.to_le_bytes());
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        hasher.update(timestamp.to_le_bytes());
        
        let payload_hash = hasher.finalize();
        let payload_to_sign = payload_hash.as_slice();
        
        // ถ้ามีการระบุ wallet และรหัสผ่าน ให้ลงนาม payload
        let (signature, wallet_address) = match get_main_wallet() {
            Some(wallet_addr) => {
                // ขอรหัสผ่าน
                let password = match &self.password {
                    Some(pwd) => pwd.clone(),
                    None => {
                        let display_addr = if wallet_addr.starts_with("0x") {
                            wallet_addr.clone()
                        } else {
                            format!("0x{}", wallet_addr)
                        };
                        
                        println!("Enter password for wallet {}: ", display_addr);
                        rpassword::read_password().unwrap_or_default()
                    }
                };
                
                // โหลด wallet และลงนาม payload
                match load_wallet(&wallet_addr, &password) {
                    Ok(wallet) => {
                        match wallet.sign(payload_to_sign, &password) {
                            Ok(sig) => {
                                let display_addr = if wallet_addr.starts_with("0x") {
                                    wallet_addr.clone()
                                } else {
                                    format!("0x{}", wallet_addr)
                                };
                                println!("✅ Transaction signed successfully with wallet {}", display_addr);
                                (Some(sig), Some(wallet_addr.clone()))
                            },
                            Err(err) => {
                                println!("❌ ERROR: Failed to sign transaction: {}", err);
                                println!("Deployment cannot continue without a valid signature.");
                                return Err(anyhow::anyhow!("Transaction signing failed: {}", err));
                            }
                        }
                    },
                    Err(WalletError::InvalidPassword) => {
                        let display_addr = if wallet_addr.starts_with("0x") {
                            wallet_addr.clone()
                        } else {
                            format!("0x{}", wallet_addr)
                        };
                        println!("❌ ERROR: Invalid password for wallet {}", display_addr);
                        return Err(anyhow::anyhow!("Invalid wallet password"));
                    },
                    Err(err) => {
                        println!("❌ ERROR: Failed to load wallet: {}", err);
                        return Err(anyhow::anyhow!("Wallet loading failed: {}", err));
                    }
                }
            },
            None => {
                // Check if an address was explicitly specified
                if self.address.is_some() {
                    // Only continue if no wallet is needed (address specified directly)
                    println!("ℹ️ No wallet configured. Publishing with specified address without signature.");
                    (None, None)
                } else {
                    println!("❌ ERROR: No wallet configured and no address specified.");
                    println!("Please specify an address with --address or configure a wallet.");
                    println!("To create a wallet, run: kari wallet create");
                    return Err(anyhow::anyhow!("No wallet configured and no address specified"));
                }
            }
        };
        
        // Create timer to measure deployment duration
        let start_time = std::time::Instant::now();
        
        println!("\n⏳ Compiling and deploying modules to blockchain...");
        
        // ส่งข้อมูลลงนามไปกับการ execute
        let mona_vm_publish = mona_vm::Publish {
            signature: signature,
            signer_address: wallet_address,
        };
        
        // Set a timeout for the deployment
        let timeout = std::time::Duration::from_secs(30); // 30 seconds timeout
        
        // Clone package_path before moving into the thread to avoid ownership issues
        let package_path_clone = package_path.clone();
        
        // Spawn a thread to execute the deployment
        let deployment_thread = std::thread::spawn(move || {
            mona_vm_publish.execute(
                Some(package_path_clone), // Use the cloned path
                Some(address),
                build_config,
                Some(self.gas_budget),
                self.skip_verify
            )
        });
        
        // Simple timed wait with status updates
        let mut elapsed = std::time::Duration::new(0, 0);
        let interval = std::time::Duration::from_secs(5);
        
        while elapsed < timeout {
            if deployment_thread.is_finished() {
                break;
            }
            std::thread::sleep(interval);
            elapsed += interval;
            println!("⏳ Still working... ({:?} elapsed)", elapsed);
        }
        
        // Check if completed or timed out
        if deployment_thread.is_finished() {
            match deployment_thread.join() {
                Ok(result) => {
                    match result {
                        Ok(()) => {
                            println!("✅ Deployment successful!");
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
                                    "package_path": package_path.to_string_lossy(), // Now we can use the original package_path
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
                },
                Err(_) => {
                    println!("❌ Deployment thread panicked");
                    Err(anyhow::anyhow!("Deployment thread panicked"))
                }
            }
        } else {
            println!("⚠️ Deployment timed out after {:?}", timeout);
            println!("💡 The deployment might still be processing in the background.");
            println!("💡 You can check if your modules were deployed with: kari move call");
            
            Err(anyhow::anyhow!("Deployment timed out after {:?}", timeout))
        }
    }
}
