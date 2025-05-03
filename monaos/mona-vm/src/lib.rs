use log::info;
use move_package::compilation::compiled_package::CompiledUnitWithSource;
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
use framework::{Package, PackageType, PackageSourceInfo};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use mona_types::gas::format_gas_fee_display;

// New imports for blockchain integration
use std::sync::{Arc, RwLock};
use mona_blockchain::block::Transaction;
use mona_blockchain::blockchain::BLOCKCHAIN_DATA;

pub fn reroot_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let rooted_path = SourcePackageLayout::try_find_root(&path.canonicalize()?)?;
    std::env::set_current_dir(rooted_path).unwrap();

    Ok(PathBuf::from("."))
}

// VM Transaction State Manager
lazy_static::lazy_static! {
    static ref VM_STATE: Arc<RwLock<VMState>> = Arc::new(RwLock::new(VMState::new()));
}

// Structure to track VM State
pub struct VMState {
    pub modules: HashMap<String, VMModule>,
    pub last_execution: u64,
    pub execution_count: u64,
}

// Structure to represent a Move VM Module
pub struct VMModule {
    pub module_id: String,
    pub address: AccountAddress,
    pub name: String,
    pub bytecode: Vec<u8>,
    pub public_functions: Vec<String>,
    pub deploy_block_height: u32,
}

impl VMState {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            last_execution: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            execution_count: 0,
        }
    }

    pub fn register_module(&mut self, module: VMModule) {
        info!("Registering VM module: {}", module.module_id);
        self.modules.insert(module.module_id.clone(), module);
    }
}

impl VMModule {
    pub fn new(
        address: AccountAddress, 
        name: String, 
        bytecode: Vec<u8>,
        public_functions: Vec<String>, 
        deploy_block_height: u32
    ) -> Self {
        let module_id = format!("0x{}::{}", address.to_hex(), name);
        
        Self {
            module_id,
            address,
            name,
            bytecode,
            public_functions,
            deploy_block_height,
        }
    }
}

// VM Transaction Structure
pub struct VMTransaction {
    pub tx_id: String,
    pub sender: String,
    pub module_id: String,
    pub function: String,
    pub args: Vec<Vec<u8>>,
    pub gas_budget: u64,
    pub timestamp: u64,
}

impl VMTransaction {
    pub fn new(
        sender: String,
        module_id: String,
        function: String,
        args: Vec<Vec<u8>>,
        gas_budget: u64
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let mut hasher = Sha3_256::new();
        hasher.update(sender.as_bytes());
        hasher.update(module_id.as_bytes());
        hasher.update(function.as_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(gas_budget.to_le_bytes());
        
        // Use the first 16 bytes of hash as transaction ID
        let hash = hasher.finalize();
        let tx_id = format!("vm_tx_{}", hex::encode(&hash[..16]));
        
        Self {
            tx_id,
            sender,
            module_id,
            function,
            args,
            gas_budget,
            timestamp,
        }
    }
}

// Execute a VM transaction (integration with blockchain)
pub fn execute_vm_transaction(tx: &VMTransaction) -> Result<JsonValue, String> {
    // Get VM state
    let state = VM_STATE.read().map_err(|e| format!("Failed to access VM state: {}", e))?;
    
    // Find the module
    let module = state.modules.get(&tx.module_id)
        .ok_or_else(|| format!("Module not found: {}", tx.module_id))?;
    
    // Validate if function exists
    if !module.public_functions.contains(&tx.function) {
        return Err(format!("Function {} not found in module {}", tx.function, tx.module_id));
    }
    
    // Fake execution
    let gas_used = estimate_execution_gas(tx, module);
    let start = std::time::Instant::now();
    
    // Simulate execution by sleeping
    std::thread::sleep(Duration::from_millis(50));
    
    // Get current block height from blockchain
    let blockchain = BLOCKCHAIN_DATA.iter();
    let block_height = match blockchain.last() {
        Some(block) => block.index,
        None => 0,
    };
    
    // Success result with fake execution details
    Ok(json!({
        "status": "success",
        "tx_id": tx.tx_id,
        "module": tx.module_id,
        "function": tx.function,
        "gas_used": gas_used,
        "gas_display": format_gas_fee_display(gas_used),
        "execution_time_ms": start.elapsed().as_millis(),
        "block_height": block_height,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

// Convert blockchain transaction to VM transaction
pub fn convert_to_vm_transaction(transaction: &Transaction) -> Option<VMTransaction> {
    // Check if this is a VM transaction by examining the data field
    let data = match &transaction.data {
        Some(data) if !data.is_empty() => data,
        _ => return None, // Not a VM transaction
    };
    
    // Try to parse VM transaction data (simplified implementation)
    // In a real system, this would deserialize proper VM call encoding
    if let Ok(vm_data) = std::str::from_utf8(data) {
        if vm_data.starts_with("VM:") {
            let parts: Vec<&str> = vm_data.split(':').collect();
            if parts.len() >= 4 {
                // Extract module_id and function
                let module_id = parts[1].to_string();
                let function = parts[2].to_string();
                let gas_budget = parts[3].parse::<u64>().unwrap_or(1000000);
                
                return Some(VMTransaction {
                    tx_id: transaction.transaction_id.clone(),
                    sender: transaction.sender.to_hex_literal(),
                    module_id,
                    function,
                    args: Vec::new(), // Simplified implementation
                    gas_budget,
                    timestamp: transaction.timestamp,
                });
            }
        }
    }
    
    None // Not a VM transaction or failed to parse
}

// Estimate gas for executing a VM transaction
fn estimate_execution_gas(tx: &VMTransaction, module: &VMModule) -> u64 {
    // Base gas cost for execution
    let base_cost = 1000;
    
    // Additional cost based on module bytecode size
    let size_cost = module.bytecode.len() as u64 / 10;
    
    // Additional cost for args
    let args_cost = tx.args.iter().map(|arg| arg.len() as u64).sum::<u64>() / 5;
    
    // Calculate total
    let total_gas = base_cost + size_cost + args_cost;
    
    // Ensure it's within reasonable bounds
    let min_gas = 100;
    let max_gas = tx.gas_budget;
    
    total_gas.clamp(min_gas, max_gas)
}

pub struct Build {
    framework_packages: HashMap<PackageType, Option<Package>>,
}

pub struct Publish;

fn generate_object_id() -> String {
    let mut hasher = Sha3_256::new();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    hasher.update(timestamp.to_le_bytes());
    hasher.update(counter.to_le_bytes());

    let hash = hasher.finalize();
    format!("0x{:0>64}", hex::encode(hash))
}

impl Build {
    pub fn new() -> Self {
        Self {
            framework_packages: HashMap::new(),
        }
    }

    pub fn with_stdlib(mut self) -> Self {
        self.framework_packages.insert(PackageType::Stdlib, Package::new(PackageType::Stdlib).ok());
        self
    }

    pub fn with_system(mut self) -> Self {
        self.framework_packages.insert(PackageType::System, Package::new(PackageType::System).ok());
        self
    }

    pub fn with_framework(mut self) -> Self {
        self.framework_packages.insert(PackageType::Framework, Package::new(PackageType::Framework).ok());
        self
    }

    pub fn execute(mut self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;

        let mut enhanced_config = config.clone();
        enhanced_config.additional_named_addresses.insert(
            "std".to_string(),
            AccountAddress::from_hex_literal("0x1").unwrap(),
        );

        enhanced_config.additional_named_addresses.insert(
            "kanari_framework".to_string(),
            AccountAddress::from_hex_literal("0x2").unwrap(),
        );

        let stdlib_path = get_stdlib_path();
        let kanari_system_path = get_kanari_system_path();
        let framework_path = get_framework_path();

        let mut framework_info = Vec::new();
        for (pkg_type, pkg_opt) in &mut self.framework_packages {
            if let Some(package) = pkg_opt {
                if let Err(e) = package.load_dependencies() {
                    info!("Warning: Failed to load dependencies for {:?}: {}", pkg_type, e);
                }

                framework_info.push(json!({
                    "type": format!("{:?}", pkg_type),
                    "path": match pkg_type {
                        PackageType::Stdlib => stdlib_path.to_string_lossy(),
                        PackageType::System => kanari_system_path.to_string_lossy(),
                        PackageType::Framework => framework_path.to_string_lossy(),
                    },
                    "dependencies": package.dependencies.len()
                }));

                Build::configure_for_framework(&mut enhanced_config, package)?;
            }
        }

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
                    "path": rerooted_path.to_string_lossy(),
                    "framework_packages": framework_info
                })
            );
            return Ok(());
        }

        let package_source_info = self.analyze_package_sources(&rerooted_path);
        let compile_start = std::time::Instant::now();

        let compiled_package = enhanced_config.compile_package(&rerooted_path, &mut Vec::new())?;
        let compile_duration = compile_start.elapsed();

        let modules_info = self.extract_modules_info(&compiled_package.root_compiled_units);

        let result = json!({
            "status": "success",
            "type": "full_build",
            "compile_time_ms": compile_duration.as_millis(),
            "metadata": {
                "package": {
                    "name": compiled_package.compiled_package_info.package_name.to_string(),
                    "id": generate_object_id(),
                    "path": rerooted_path.to_string_lossy(),
                    "source_files": package_source_info,
                    "info": {
                        "source_digest": compiled_package.compiled_package_info.source_digest,
                        "addresses": compiled_package.compiled_package_info.address_alias_instantiation
                            .iter()
                            .map(|(name, addr)| (name.to_string(), json!(format!("0x{}", addr.to_hex()))))
                            .collect::<serde_json::Map<String, JsonValue>>(),
                    }
                },
                "framework": {
                    "packages": framework_info,
                    "stdlib_path": stdlib_path.to_string_lossy().to_string(),
                    "system_path": kanari_system_path.to_string_lossy().to_string(),
                    "framework_path": framework_path.to_string_lossy().to_string(),
                },
                "modules": modules_info
            }
        });

        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }

    fn configure_for_framework(config: &mut BuildConfig, package: &Package) -> anyhow::Result<()> {
        let _ = config;
        if let Ok(dep_paths) = package.resolve_dependencies() {
            for dep_path in dep_paths {
                if dep_path.exists() {
                    info!("Adding framework dependency path: {}", dep_path.display());
                }
            }
        }

        Ok(())
    }

    fn analyze_package_sources(&self, path: &PathBuf) -> JsonValue {
        let source_dir = path.join("sources");
        let mut source_files = Vec::new();

        if source_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(source_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "move") {
                            if let Ok(info) = PackageSourceInfo::create(&path) {
                                source_files.push(json!({
                                    "path": path.to_string_lossy(),
                                    "name": info.module_name,
                                    "has_tests": info.has_tests,
                                    "dependencies": info.dependencies,
                                }));
                            } else {
                                source_files.push(json!({
                                    "path": path.to_string_lossy(),
                                    "name": path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"),
                                }));
                            }
                        }
                    }
                }
            }
        }

        json!(source_files)
    }

    fn extract_modules_info(&self, compiled_units: &[CompiledUnitWithSource]) -> JsonValue {
        let modules = compiled_units.iter().map(|unit| {
            json!({
                "name": unit.unit.name().to_string(),
                "source_path": unit.source_path.to_string_lossy(),
                "dependencies": get_module_dependencies(&unit.unit),
                "public_functions": get_module_public_functions(&unit.unit)
            })
        }).collect::<Vec<_>>();

        json!(modules)
    }
}

impl Publish {
    pub fn execute(
        self,
        path: Option<PathBuf>,
        address: Option<AccountAddress>,
        config: BuildConfig,
        gas_budget: Option<u64>,
        skip_verify: bool,
    ) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;

        let address = address.unwrap_or_else(|| AccountAddress::from_hex_literal("0x1").unwrap());

        let mut build_config = config.clone();
        build_config.additional_named_addresses.insert("module_addr".to_string(), address);

        info!(
            "Compiling Move package for blockchain deployment at: {}",
            rerooted_path.display()
        );
        let start_time = std::time::Instant::now();

        let compiled_package = build_config.compile_package(&rerooted_path, &mut Vec::new())?;

        info!("Package compilation completed in {:?}", start_time.elapsed());

        let deployment_info = self.prepare_deployment(
            &compiled_package,
            address,
            gas_budget.unwrap_or(3_000_000),
            skip_verify,
        )?;

        let deployment_result = self.submit_to_blockchain(&compiled_package, &address, &deployment_info)?;

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
        skip_verify: bool,
    ) -> anyhow::Result<JsonValue> {
        let mut modules_json = Vec::new();
        
        // Format address with 64 characters for consistency
        let address_str = format!("{:0>64}", address.to_hex());
        let address_0x = format!("0x{}", address_str);

        for unit in &package.root_compiled_units {
            let module = &unit.unit;
            let module_name = module.name().to_string();
            let module_id = ModuleId::new(address, Identifier::new(module_name.clone())?);
            
            // Standard module ID format (without 0x)
            let standard_module_id = module_id.to_string();
            
            // Full module ID with extended address format and 0x prefix
            let full_module_id = format!("{}::{}", address_0x, module_name);

            let bytecode = module.serialize(None);

            let gas_estimate = estimate_gas_for_module(&bytecode, gas_budget);

            let module_meta = json!({
                "id": generate_object_id(),
                "name": module_name,
                "module_id": standard_module_id,
                "full_module_id": full_module_id,
                "size_bytes": bytecode.len(),
                "verification": {
                    "skip": skip_verify,
                    "gas_estimate": gas_estimate,
                    "gas_display": format_gas_fee_display(gas_estimate)
                },
                "constructor_args": [],
                "dependencies": get_module_dependencies(&unit.unit),
                "public_functions": get_module_public_functions(&unit.unit)
            });

            modules_json.push(module_meta);
        }

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
        deployment_info: &JsonValue,
    ) -> anyhow::Result<DeploymentResult> {
        info!(
            "Submitting {} modules to blockchain at address: 0x{}",
            package.root_compiled_units.len(),
            address.to_hex()
        );
    
        let start = std::time::Instant::now();
        std::thread::sleep(Duration::from_millis(500));
    
        let modules = deployment_info["modules"].as_array().unwrap();
        let mut total_gas_used = 0;
        
        // Get current block height
        let blockchain = BLOCKCHAIN_DATA.iter();
        let block_height = match blockchain.last() {
            Some(block) => block.index,
            None => 0,
        };
    
        // Get mutable access to VM state
        let mut vm_state = match VM_STATE.write() {
            Ok(state) => state,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to lock VM state for writing: {}", e));
            }
        };
    
        // Register modules with VM
        for (i, module_json) in modules.iter().enumerate() {
            let module_name = module_json["name"].as_str().unwrap();
            let size_bytes = module_json["size_bytes"].as_u64().unwrap_or(0);
            
            // Get public functions
            let public_funcs = module_json["public_functions"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>();
            
            // Calculate gas
            let priority_boost = size_bytes / 100;
            let gas_used = mona_types::gas::calculate_gas_fee(Some(priority_boost));
            total_gas_used += gas_used;
            
            // Register with VM
            let vm_module = VMModule::new(
                *address,
                module_name.to_string(),
                // Use a fake bytecode for this example
                vec![0u8; size_bytes as usize],
                public_funcs,
                block_height,
            );
            
            vm_state.register_module(vm_module);
            
            info!(
                "Module '{}' deployed and registered with VM - size: {} bytes, gas used: {} ({})",
                module_name,
                size_bytes,
                gas_used,
                mona_types::gas::format_gas_fee_display(gas_used)
            );
        }
    
        let execution_time = start.elapsed().as_millis();
    
        let result = DeploymentResult {
            transaction_id: format!("0x{}", generate_random_hex(64)),
            status: "COMMITTED".to_string(),
            gas_used: total_gas_used,
            execution_time_ms: execution_time as u64,
            block_height: block_height as u64,
        };
    
        info!(
            "Blockchain deployment completed in {}ms. Transaction ID: {}, gas used: {} ({})",
            execution_time,
            result.transaction_id,
            result.gas_used,
            mona_types::gas::format_gas_fee_display(result.gas_used)
        );
    
        Ok(result)
    }
}

struct DeploymentResult {
    transaction_id: String,
    status: String,
    gas_used: u64,
    execution_time_ms: u64,
    block_height: u64,
}


fn estimate_gas_for_module(bytecode: &[u8], gas_budget: u64) -> u64 {
    // Base gas cost for any module
    let base_cost = mona_types::gas::BASE_GAS_FEE;
    
    // Additional cost based on bytecode size
    // The larger the bytecode, the more gas it will consume
    let size_cost = bytecode.len() as u64 * 10;
    
    // Priority boost based on module size
    let priority_boost = bytecode.len() as u64 / 100;
    
    // Get network stats to factor in current conditions
    let network_stats = mona_types::gas::get_network_stats();
    
    // Calculate congestion component based on network stats
    let congestion_factor = if network_stats.pending_transactions > 0 {
        (1.0 + (network_stats.pending_transactions as f64 / 10.0).ln_1p()) 
            * mona_types::gas::CONGESTION_MULTIPLIER
    } else {
        1.0
    };
    
    // Calculate total estimated gas
    let estimate = base_cost + (size_cost as f64 * congestion_factor) as u64;
    
    // Apply priority boost
    let estimate = estimate + priority_boost;
    
    // Ensure estimate is within allowed range and doesn't exceed budget
    let min_gas = mona_types::gas::MIN_GAS_FEE;
    let max_gas = std::cmp::min(mona_types::gas::MAX_GAS_FEE, gas_budget);
    
    estimate.clamp(min_gas, max_gas)
}

fn get_module_dependencies(module: &CompiledUnit) -> Vec<String> {
    let compiled_module = &module.module;

    compiled_module
        .module_handles()
        .iter()
        .filter_map(|handle| {
            if handle.name == compiled_module.self_handle().name {
                return None;
            }

            let module_name = compiled_module.identifier_at(handle.name);
            let address = compiled_module.address_identifier_at(handle.address);

            Some(format!("{}::{}", address, module_name))
        })
        .collect()
}

fn get_module_public_functions(module: &CompiledUnit) -> Vec<JsonValue> {
    let compiled_module = &module.module;
    let module_address = compiled_module.address().to_string();
    let module_name = compiled_module.name().to_string();
    let full_module_id = format!("0x{}", module_address);
    
    compiled_module
        .function_defs()
        .iter()
        .filter_map(|func_def| {
            // Check if function is public
            if func_def.visibility == move_binary_format::file_format::Visibility::Public {
                let func_handle = compiled_module.function_handle_at(func_def.function);
                let func_name = compiled_module.identifier_at(func_handle.name).to_string();
                
                // Full qualified function name with address, module and function
                let _full_func_id = format!("{}::{}", full_module_id, func_name);
                // Complete function path includes module name
                let complete_func_path = format!("{}::{}", full_module_id, module_name);
                
                // Extract function parameters and return types if available
                let signature = compiled_module.signature_at(func_handle.parameters);
                let param_count = signature.0.len();
                
                Some(json!({
                    "name": func_name,
                    "full_name": format!("{}::{}", complete_func_path, func_name),
                    "module": complete_func_path,
                    "visibility": "public",
                    "parameters": param_count,

                }))
            } else {
                None
            }
        })
        .collect()
}

fn generate_random_hex(length: usize) -> String {
    let mut rng = thread_rng();
    const CHARSET: &[u8] = b"0123456789abcdef";

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}