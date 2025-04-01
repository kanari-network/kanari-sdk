use move_package::{source_package::layout::SourcePackageLayout, BuildConfig};
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;
use sha3::{Digest, Sha3_256};
use std::time::{SystemTime, UNIX_EPOCH};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;

use move_core_types::identifier::Identifier;


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
        if config.fetch_deps_only {
            let mut config = config;
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

        let compiled_package = config
            .clone()
            .compile_package(&rerooted_path, &mut Vec::new())?;

        // Enhanced metadata JSON output with detailed function info
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
        
        let compiled_package = build_config
            .compile_package(&rerooted_path, &mut Vec::new())?;
        
        // Create deployment metadata
        let deployment_info = self.prepare_deployment(
            &compiled_package, 
            address,
            gas_budget.unwrap_or(10000000), 
            skip_verify
        )?;
        
        // Output deployment JSON
        let result = json!({
            "status": "success",
            "type": "deployment",
            "metadata": {
                "package": {
                    "name": compiled_package.compiled_package_info.package_name.to_string(),
                    "id": generate_object_id(),
                    "path": rerooted_path.to_string_lossy(),
                    "address": format!("0x{}", address.to_hex()),
                    "gas_budget": gas_budget.unwrap_or(10000000)
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
            
            // Get compiled bytecode - Fix: provide None as argument and remove ? operator
            let bytecode = module.serialize(None);
            
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
                    "gas_estimate": estimate_gas_for_module(&bytecode, gas_budget)
                }
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
}

/// Estimate gas for module deployment
/// This is a placeholder function. In a real implementation, you would
fn estimate_gas_for_module(bytecode: &[u8], max_gas: u64) -> u64 {
    // Simple estimation heuristic - adjust as needed
    // This is a placeholder. Real gas estimation would be more complex
    let base_cost = 1000;
    let size_multiplier = 10;
    let estimated = base_cost + (bytecode.len() as u64 * size_multiplier);
    
    std::cmp::min(estimated, max_gas)
}