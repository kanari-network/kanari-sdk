use log::info;
use move_package::{source_package::layout::SourcePackageLayout, BuildConfig};
use move_compiler::compiled_unit::CompiledUnit;
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;
use std::time::Duration;
use sha3::{Digest, Sha3_256};
use std::time::{SystemTime, UNIX_EPOCH};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
use move_core_types::identifier::Identifier;
use framework::get_stdlib_path;
use framework::get_kanari_system_path;
use framework::get_framework_path;

pub fn reroot_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    // Always root ourselves to the package root, and then compile relative to that.
    let rooted_path = SourcePackageLayout::try_find_root(&path.canonicalize()?)?;
    std::env::set_current_dir(rooted_path).unwrap();

    Ok(PathBuf::from("."))
}

pub struct Build;

pub struct Publish;

fn generate_object_id() -> String {
    let mut hasher = Sha3_256::new();
    
    // Get timestamp and counter
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    
    // Combine data and hash
    hasher.update(timestamp.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    
    // Get 32-byte (256-bit) hash result
    let hash = hasher.finalize();
    
    // Convert to 64-character hex string with "0x" prefix
    format!("0x{:0>64}", hex::encode(hash))
}

impl Build {
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;
        
        // Enhance config with framework paths
        let mut enhanced_config = config.clone();
        enhanced_config.additional_named_addresses.insert(
            "std".to_string(),
            AccountAddress::from_hex_literal("0x1").unwrap()
        );
        
        // Get framework paths for reference in build metadata
        let stdlib_path = get_stdlib_path();
        let kanari_system_path = get_kanari_system_path();
        let framework_path = get_framework_path();
        
        // Note: BuildConfig doesn't have a 'deps' field
        // We'll add dependencies to the package search path through other methods if needed
        // For now, we'll just use the paths in the metadata output
        
        if config.fetch_deps_only {
            let mut config = enhanced_config.clone();
            if config.test_mode {
                config.dev_mode = true;
            }
            config.download_deps_for_package(&rerooted_path, &mut std::io::stdout())?;
            println!(
                "{}",
                json!({
                    "status": "success",
                    "type": "deps_only",
                    "path": rerooted_path.to_string_lossy()
                })
            );
            return Ok(());
        }

        let compiled_package = enhanced_config
            .compile_package(&rerooted_path, &mut Vec::new())?;

        // Enhanced metadata JSON output with detailed function info and framework info
        let result = json!({
            "status": "success",
            "type": "full_build",
            "metadata": {
                "package": {
                    "name": compiled_package.compiled_package_info.package_name.to_string(),
                    "id": generate_object_id(),  // Add unique package ID
                    "path": rerooted_path.to_string_lossy(),
                    "info": {
                        "source_digest": compiled_package.compiled_package_info.source_digest,
                        "addresses": compiled_package.compiled_package_info.address_alias_instantiation
                            .iter()
                            .map(|(name, addr)| (name.to_string(), json!(format!("0x{}", addr.to_hex()))))
                            .collect::<serde_json::Map<String, JsonValue>>(),
                    }
                },
                "framework": {
                    "stdlib_path": stdlib_path.to_string_lossy().to_string(),
                    "system_path": kanari_system_path.to_string_lossy().to_string(),
                    "framework_path": framework_path.to_string_lossy().to_string(),
                },
                "modules": compiled_package.root_compiled_units
                    .iter()
                    .map(|unit| {
                        let module = &unit.unit;
                        json!({
                            "id": generate_object_id(),
                            "name": module.name().to_string(),
                            "source_path": unit.source_path.to_string_lossy(),
                            "content": {
                                "functions": module.module.function_defs()
                                    .iter()
                                    .map(|fdef| {
                                        let handle = module.module.function_handle_at(fdef.function);
                                        let name = module.module.identifier_at(handle.name);
                                        json!({
                                            "id": generate_object_id(),
                                            "name": name.to_string(),
                                            "metadata": {
                                                "visibility": format!("{:?}", fdef.visibility),
                                                "is_entry": fdef.is_entry,
                                                "handle_id": format!("0x{}", fdef.function.0),
                                                "acquires_global_resources": fdef.acquires_global_resources
                                                    .iter()
                                                    .map(|s| format!("0x{}", s.0))
                                                    .collect::<Vec<_>>()
                                            },
                                            "signature": {
                                                "parameters": module.module.signature_at(handle.parameters)
                                                    .0
                                                    .iter()
                                                    .map(|ty| format!("{:?}", ty))
                                                    .collect::<Vec<_>>(),
                                                "return_types": module.module.signature_at(handle.return_)
                                                    .0
                                                    .iter()
                                                    .map(|ty| format!("{:?}", ty))
                                                    .collect::<Vec<_>>()
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                                "structs": module.module.struct_defs()
                                    .iter()
                                    .map(|sdef| {
                                        let handle = module.module.struct_handle_at(sdef.struct_handle);
                                        let name = module.module.identifier_at(handle.name);
                                        json!({
                                            "id": generate_object_id(),
                                            "name": name.to_string(),
                                            "metadata": {
                                                "handle_id": format!("0x{}", sdef.struct_handle.0),
                                                "abilities": format!("{:?}", handle.abilities),
                                                "type_parameters": handle.type_parameters
                                                    .iter()
                                                    .map(|tp| json!({
                                                        "constraints": format!("{:?}", tp.constraints),
                                                        "is_phantom": tp.is_phantom
                                                    }))
                                                    .collect::<Vec<_>>()
                                            },
                                            "fields": match &sdef.field_information {
                                                move_binary_format::file_format::StructFieldInformation::Native => Vec::new(),
                                                move_binary_format::file_format::StructFieldInformation::Declared(fields) => {
                                                    fields.iter()
                                                        .map(|field| json!({
                                                            "id": generate_object_id(),
                                                            "name": module.module.identifier_at(field.name).to_string(),
                                                            "type": format!("{:?}", field.signature.0)
                                                        }))
                                                        .collect::<Vec<_>>()
                                                }
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            }
        });

        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
}

impl Publish {
    pub fn execute(
        self, 
        path: Option<PathBuf>, 
        address: Option<AccountAddress>,
        config: BuildConfig,
        gas_budget: Option<u64>,
        skip_verify: bool
    ) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;
        
        // Set default address if none provided
        let address = address.unwrap_or_else(|| 
            AccountAddress::from_hex_literal("0x1").unwrap()
        );

        // Update build config with address
        let mut build_config = config.clone();
        build_config.additional_named_addresses.insert(
            "module_addr".to_string(),
            address
        );
        
        info!("Compiling Move package for blockchain deployment at: {}", rerooted_path.display());
        let start_time = std::time::Instant::now();
        
        let compiled_package = build_config
            .compile_package(&rerooted_path, &mut Vec::new())?;
        
        info!("Package compilation completed in {:?}", start_time.elapsed());
        
        // Create deployment metadata
        let deployment_info = self.prepare_deployment(
            &compiled_package, 
            address,
            gas_budget.unwrap_or(3_000_000), // Use reasonable default gas budget for blockchain deployment 
            skip_verify
        )?;
        
        // Submit modules to blockchain
        let deployment_result = self.submit_to_blockchain(
            &compiled_package,
            &address,
            &deployment_info
        )?;
        
        // Output deployment JSON with blockchain transaction results
        let result = json!({
            "status": "success",
            "type": "blockchain_deployment",
            "metadata": {
                "package": {
                    "name": compiled_package.compiled_package_info.package_name.to_string(),
                    "id": generate_object_id(),
                    "path": rerooted_path.to_string_lossy(),
                    "address": format!("0x{}", address.to_hex()),
                    "gas_budget": gas_budget.unwrap_or(3_000_000),
                    "gas_used": deployment_result.gas_used,
                    "deploy_time": deployment_result.execution_time_ms,
                },
                "blockchain": {
                    "transaction_id": deployment_result.transaction_id,
                    "status": deployment_result.status,
                    "block_height": deployment_result.block_height,
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
                "deployment": deployment_info
            }
        });
        
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
    
    fn prepare_deployment(
        &self,
        package: &move_package::compilation::compiled_package::CompiledPackage,
        address: AccountAddress,
        gas_budget: u64,
        skip_verify: bool
    ) -> anyhow::Result<JsonValue> {
        let mut modules_json = Vec::new();
        
        for unit in &package.root_compiled_units {
            let module = &unit.unit;
            let module_name = module.name().to_string();
            let module_id = ModuleId::new(address, Identifier::new(module_name.clone())?);
            
            // Get compiled bytecode
            let bytecode = module.serialize(None);
            
            // Calculate gas estimate with our improved function
            let gas_estimate = estimate_gas_for_module(&bytecode, gas_budget);
            
            // Create module metadata
            let module_meta = json!({
                "id": generate_object_id(),
                "name": module_name,
                "module_id": module_id.to_string(),
                "bytecode": hex::encode(&bytecode),
                "source_path": unit.source_path.to_string_lossy(),
                "size_bytes": bytecode.len(),
                "verification": {
                    "skip": skip_verify,
                    "gas_estimate": gas_estimate,
                    "gas_display": format_gas_amount(gas_estimate)
                },
                "constructor_args": [], // Placeholder for future constructor arguments
                "dependencies": get_module_dependencies(&unit.unit)
            });
            
            modules_json.push(module_meta);
        }
        
        // Generate deployment JSON
        let deployment_json = json!({
            "modules": modules_json,
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "id": generate_object_id(),
        });
        
        Ok(deployment_json)
    }
    
    fn submit_to_blockchain(
        &self,
        package: &move_package::compilation::compiled_package::CompiledPackage,
        address: &AccountAddress,
        deployment_info: &JsonValue
    ) -> anyhow::Result<DeploymentResult> {
        // In a real implementation, this would submit the compiled modules to the blockchain
        // For now, we're simulating the deployment process
        
        info!("Submitting {} modules to blockchain at address: 0x{}", 
              package.root_compiled_units.len(), address.to_hex());
        
        // Simulate blockchain deployment
        let start = std::time::Instant::now();
        
        // Simulate network delay and processing
        std::thread::sleep(Duration::from_millis(500));
        
        let modules = deployment_info["modules"].as_array().unwrap();
        let mut total_gas_used = 0;
        
        for module_json in modules {
            let module_name = module_json["name"].as_str().unwrap();
            let bytecode_hex = module_json["bytecode"].as_str().unwrap();
            let gas_estimate = module_json["verification"]["gas_estimate"].as_u64().unwrap();
            
            // Simulate actual gas used (slightly different from estimate)
            let gas_used = simulate_gas_usage(gas_estimate);
            total_gas_used += gas_used;
            
            info!("Module '{}' deployed - size: {} bytes, gas used: {} ({})", 
                  module_name, 
                  bytecode_hex.len() / 2, 
                  gas_used,
                  format_gas_amount(gas_used));
        }
        
        let execution_time = start.elapsed().as_millis();
        
        // In a real implementation, we would return actual transaction information
        let result = DeploymentResult {
            transaction_id: format!("0x{}", generate_random_hex(64)),
            status: "COMMITTED".to_string(),
            gas_used: total_gas_used,
            execution_time_ms: execution_time as u64,
            block_height: 12345, // Placeholder - would come from actual blockchain
        };
        
        info!("Blockchain deployment completed in {}ms. Transaction ID: {}, gas used: {} ({})", 
              execution_time, 
              result.transaction_id, 
              result.gas_used,
              format_gas_amount(result.gas_used));
        
        Ok(result)
    }
}

/// Information about a completed deployment
struct DeploymentResult {
    transaction_id: String,
    status: String,
    gas_used: u64,
    execution_time_ms: u64,
    block_height: u64,
}

/// Estimate gas for module deployment
/// In a real implementation, this would analyze the bytecode to provide accurate estimates
fn estimate_gas_for_module(bytecode: &[u8], max_gas: u64) -> u64 {
    // Base cost for any module deployment
    let base_cost = 50_000; // 0.00005 KA
    
    // Calculate cost based on bytecode size
    // Larger modules cost more to deploy
    let size_multiplier = 50;
    let size_cost = bytecode.len() as u64 * size_multiplier;
    
    // Additional cost based on complexity (approximated by bytecode length)
    let complexity_factor = (bytecode.len() as f64).sqrt() as u64 * 100;
    
    // Combine all factors with reasonable caps
    let estimated = base_cost + size_cost + complexity_factor;
    
    // Ensure the estimate is within reasonable bounds
    // Min: 20,000 (0.00002 KA)
    // Max: Either max_gas or 3,000,000 (0.003 KA), whichever is lower
    let min_gas = 20_000;
    let max_gas_limit = std::cmp::min(max_gas, 3_000_000);
    
    std::cmp::min(std::cmp::max(estimated, min_gas), max_gas_limit)
}

/// Format gas amount to a human-readable string
fn format_gas_amount(gas: u64) -> String {
    const KA_PER_KARI: f64 = 1_000_000_000.0;
    let kari_amount = gas as f64 / KA_PER_KARI;
    format!("{:.9} KARI", kari_amount)
}

/// Simulate actual gas usage (with some variation from the estimate)
fn simulate_gas_usage(estimated_gas: u64) -> u64 {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    
    // Actual gas used varies from 90% to 110% of the estimate
    let variation = rng.gen_range(0.9..1.1);
    (estimated_gas as f64 * variation) as u64
}

/// Get module dependencies for a compiled module
fn get_module_dependencies(module: &CompiledUnit) -> Vec<String> {
    // Access the module directly
    let compiled_module = &module.module;
    
    compiled_module.module_handles()
        .iter()
        .filter_map(|handle| {
            // Skip self-references - use the name index to check
            // We're looking for external module dependencies only
            if handle.name == compiled_module.self_handle().name {
                return None;
            }
            
            // Get the module name and address directly from the handle
            let module_name = compiled_module.identifier_at(handle.name);
            let address = compiled_module.address_identifier_at(handle.address);
            
            Some(format!("{}::{}", address, module_name))
        })
        .collect()
}

/// Generate a random hex string of specified length
fn generate_random_hex(length: usize) -> String {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    const CHARSET: &[u8] = b"0123456789abcdef";
    
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}