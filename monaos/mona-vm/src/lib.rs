use log::{info, warn};
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
use mona_blockchain::blockchain::{BLOCKCHAIN_DATA, submit_transaction};
use lazy_static::lazy_static;
use mona_crypto::verify_signature;

// Add database module
pub mod db;
use db::{store_module, store_transaction};

pub fn reroot_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let rooted_path = SourcePackageLayout::try_find_root(&path.canonicalize()?)?;
    std::env::set_current_dir(rooted_path).unwrap();

    Ok(PathBuf::from("."))
}

// VM Transaction State Manager - Make it public so it can be accessed by the RPC API
lazy_static! {
    pub static ref VM_STATE: Arc<RwLock<VMState>> = {
        // Initialize VM_STATE with modules from database
        let state = VMState::new();
        
        // Create and return the Arc<RwLock<VMState>>
        let state_arc = Arc::new(RwLock::new(state));
        
        // Load modules from database in a background thread to avoid blocking init
        std::thread::spawn({
            let state_arc = state_arc.clone();
            move || if let Err(e) = load_modules_from_db(&state_arc) {
                log::error!("Failed to load modules from database: {}", e);
            }
        });
        
        state_arc
    };
}

// New function to load modules from database
fn load_modules_from_db(state_arc: &Arc<RwLock<VMState>>) -> Result<(), String> {
    log::info!("Loading Move modules from database...");
    
    match db::list_modules(1000, 0) {
        Ok(modules) => {
            log::info!("Found {} modules in database", modules.len());
            
            if let Ok(mut state) = state_arc.write() {
                for stored_module in modules {
                    // Parse address from stored module
                    let address_str = if stored_module.address.starts_with("0x") {
                        &stored_module.address[2..]
                    } else {
                        &stored_module.address
                    };
                    
                    match AccountAddress::from_hex(address_str) {
                        Ok(address) => {
                            log::info!(
                                "Registering module {}::{} ({})",
                                address_str,
                                stored_module.name,
                                stored_module.module_id
                            );
                            
                            // Extract public functions from ABI
                            let public_functions = match serde_json::from_str::<serde_json::Value>(&stored_module.abi) {
                                Ok(abi) => {
                                    if let Some(funcs) = abi["public_functions"].as_array() {
                                        funcs
                                            .iter()
                                            .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                                            .collect()
                                    } else {
                                        Vec::new()
                                    }
                                },
                                Err(_) => Vec::new()
                            };
                            
                            // Create and register module
                            let vm_module = VMModule::new(
                                address,
                                stored_module.name,
                                stored_module.bytecode,
                                public_functions,
                                stored_module.deploy_block_height,
                            );
                            
                            state.register_module(vm_module);
                        },
                        Err(e) => {
                            log::warn!("Failed to parse address for module {}: {}", stored_module.module_id, e);
                        }
                    }
                }
                
                log::info!("Successfully loaded {} modules into VM state", state.modules.len());
                Ok(())
            } else {
                Err("Failed to acquire write lock on VM_STATE".into())
            }
        },
        Err(e) => {
            Err(format!("Failed to list modules from database: {:?}", e))
        }
    }
}

// Structure to track VM State
pub struct VMState {
    pub modules: HashMap<String, VMModule>,
    pub last_execution: u64,
    pub execution_count: u64,
    pub last_signature: Option<String>,
    pub last_signer: Option<String>,
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
            last_signature: None,
            last_signer: None,
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
        
        log::debug!("Creating VM module with ID: {}", module_id);
        
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
#[derive(Debug)]  // Add Debug trait implementation
pub struct VMTransaction {
    pub tx_id: String,
    pub sender: String,
    pub module_id: String,
    pub function: String,
    pub args: Vec<Vec<u8>>,
    pub gas_budget: u64,
    pub timestamp: u64,
    pub signature: Option<Vec<u8>>,
    pub signer_address: Option<String>,
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
            signature: None,
            signer_address: None,
        }
    }
    
    pub fn with_signature(mut self, signature: Vec<u8>, signer_address: String) -> Self {
        self.signature = Some(signature);
        self.signer_address = Some(signer_address);
        self
    }
}

// Enhanced execute_vm_transaction function with detailed logging
pub fn execute_vm_transaction(tx: &VMTransaction) -> Result<JsonValue, String> {
    log::info!("Executing VM transaction: {} -> {}.{}", tx.tx_id, tx.module_id, tx.function);
    
    // Dump all registered modules for debugging
    let state = VM_STATE.read().map_err(|e| format!("Failed to access VM state: {}", e))?;
    
    log::info!("Available modules in VM state ({} total):", state.modules.len());
    for (id, module) in state.modules.iter() {
        log::info!("  - {} (address: 0x{}, functions: {})", 
                  id, module.address.to_hex(), 
                  module.public_functions.join(", "));
    }
    
    // Try direct dynamic loading from database first
    let db_module = match direct_db_module_lookup(&tx.module_id) {
        Ok(Some(module)) => {
            log::info!("Found module directly from database: {}", module.module_id);
            
            // Register the module in VM state
            if let Ok(mut vm_state) = VM_STATE.write() {
                vm_state.register_module(module.clone());
                log::info!("Dynamically registered module from database");
                Some(module)
            } else {
                None
            }
        },
        Ok(None) => None,
        Err(e) => {
            log::warn!("Error looking up module in database: {}", e);
            None
        }
    };
    
    // Try to find module using variations
    let module = match db_module {
        Some(m) => Ok(m),
        None => find_module_with_variations(&state, &tx.module_id)
    }?;
    
    // Check if function exists in the module
    if !module.public_functions.contains(&tx.function) {
        log::error!("Function '{}' not found in module {}. Available functions: {}", 
                  tx.function, tx.module_id, module.public_functions.join(", "));
        return Err(format!("Function {} not found in module {}", tx.function, tx.module_id));
    }
    
    log::info!("Found module and function, executing transaction");
    
    // Rest of the existing execution logic...
    let gas_used = estimate_execution_gas(tx, &module);
    let start = std::time::Instant::now();
    
    // Simulate execution by sleeping
    std::thread::sleep(Duration::from_millis(50));
    
    // Get current block height from blockchain
    let blockchain = BLOCKCHAIN_DATA.iter();
    let block_height = match blockchain.last() {
        Some(block) => block.index,
        None => 0,
    };
    
    // Generate result JSON
    let result = json!({
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
    });
    
    // Store transaction in database
    let result_str = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
    if let Err(e) = store_transaction(
        &tx.tx_id,
        &tx.module_id,
        &tx.function,
        &tx.args,
        &tx.sender,
        gas_used,
        true, // success status
        &result_str,
        block_height as u64
    ) {
        info!("Failed to store transaction in database: {:?}", e);
        // Continue even if database storage fails
    } else {
        info!("Transaction {} stored in database", tx.tx_id);
    }
    
    Ok(result)
}

// New function for direct database lookup bypassing VM state
fn direct_db_module_lookup(module_id: &str) -> Result<Option<VMModule>, String> {
    // Try different variations of module ID
    let variations = generate_module_id_variations(module_id);
    
    // Check database for each variation
    for id in std::iter::once(module_id.to_string()).chain(variations) {
        match db::get_module(&id) {
            Ok(Some(stored_module)) => {
                log::info!("Found module '{}' in database", stored_module.module_id);
                
                // Parse address from stored module
                let address_str = if stored_module.address.starts_with("0x") {
                    &stored_module.address[2..]
                } else {
                    &stored_module.address
                };
                
                // Create module from database record
                match AccountAddress::from_hex(address_str) {
                    Ok(address) => {
                        // Extract public functions from ABI
                        let public_functions = match serde_json::from_str::<serde_json::Value>(&stored_module.abi) {
                            Ok(abi) => {
                                if let Some(funcs) = abi["public_functions"].as_array() {
                                    funcs.iter()
                                        .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                                        .collect()
                                } else {
                                    Vec::new()
                                }
                            },
                            Err(_) => Vec::new()
                        };
                        
                        // Create VM module
                        return Ok(Some(VMModule::new(
                            address,
                            stored_module.name,
                            stored_module.bytecode,
                            public_functions,
                            stored_module.deploy_block_height,
                        )));
                    },
                    Err(e) => {
                        log::warn!("Failed to parse address for module {}: {}", stored_module.module_id, e);
                    }
                }
            },
            Ok(None) => continue,
            Err(e) => {
                log::warn!("Error querying database for module {}: {:?}", id, e);
            }
        }
    }
    
    Ok(None)
}

// Helper function to find a module with different address formats
fn find_module_with_variations(state: &VMState, module_id: &str) -> Result<VMModule, String> {
    // First try exact match
    if let Some(module) = state.modules.get(module_id) {
        return Ok(module.clone());
    }
    
    // Log available modules for debugging
    log::info!("Module '{}' not found in exact match. Available modules:", module_id);
    for (id, module) in state.modules.iter() {
        log::info!("  - {} (address: 0x{})", id, module.address.to_hex());
    }
    
    // Try with different address formats
    let variations = generate_module_id_variations(module_id);
    log::info!("Trying {} variations of module ID", variations.len());
    
    for variant in &variations {
        log::info!("Trying variation: {}", variant);
        if let Some(module) = state.modules.get(variant) {
            log::info!("Found module using alternate format: {} → {}", module_id, variant);
            return Ok(module.clone());
        }
    }
    
    // Try case-insensitive partial matching (more flexible)
    let module_id_lower = module_id.to_lowercase();
    for (id, module) in state.modules.iter() {
        if id.to_lowercase().contains(&module_id_lower) || 
           module_id_lower.contains(&id.to_lowercase()) {
            log::info!("Found module using partial match: {} → {}", module_id, id);
            return Ok(module.clone());
        }
    }
    
    // Try direct match with the database
    log::info!("Trying to load module '{}' from database", module_id);
    let results = match db::list_modules(100, 0) {
        Ok(modules) => modules,
        Err(e) => {
            log::error!("Error loading modules from database: {:?}", e);
            return Err(format!("Module not found: {} (database error)", module_id));
        }
    };
    
    // Check if any module in database matches
    for stored_module in results {
        if stored_module.module_id.to_lowercase() == module_id.to_lowercase() || 
           variations.iter().any(|v| v.to_lowercase() == stored_module.module_id.to_lowercase()) {
            
            log::info!("Found module '{}' in database, loading into VM state", stored_module.module_id);
            
            // Parse address from stored module
            let address_str = if stored_module.address.starts_with("0x") {
                &stored_module.address[2..]
            } else {
                &stored_module.address
            };
            
            // Create and register the module in VM state
            match AccountAddress::from_hex(address_str) {
                Ok(address) => {
                    // Extract public functions from ABI
                    let public_functions = match serde_json::from_str::<serde_json::Value>(&stored_module.abi) {
                        Ok(abi) => {
                            if let Some(funcs) = abi["public_functions"].as_array() {
                                funcs.iter()
                                    .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        },
                        Err(_) => Vec::new()
                    };
                    
                    // Create VM module from database record
                    let vm_module = VMModule::new(
                        address,
                        stored_module.name,
                        stored_module.bytecode,
                        public_functions,
                        stored_module.deploy_block_height,
                    );
                    
                    // Register module dynamically
                    let mut vm_state = VM_STATE.write().map_err(|e| {
                        format!("Failed to acquire write lock on VM_STATE: {}", e)
                    })?;
                    
                    vm_state.register_module(vm_module.clone());
                    log::info!("Successfully registered module {} from database", stored_module.module_id);
                    
                    // Find the module in state again (now that it's registered)
                    if let Some(module) = vm_state.modules.get(&vm_module.module_id) {
                        return Ok(module.clone());
                    }
                    
                    if let Some(module) = vm_state.modules.get(module_id) {
                        return Ok(module.clone());
                    }
                    
                    // If still not found, return the error with more context
                    return Err(format!("Module registered but not found in VM state: {}", module_id));
                },
                Err(e) => {
                    log::warn!("Failed to parse address for module {}: {}", stored_module.module_id, e);
                }
            }
        }
    }
    
    // If we still haven't found the module, log detailed diagnostics and return error
    Err(format!("Module not found after all attempts: {}", module_id))
}

// Enhanced generate_module_id_variations function
fn generate_module_id_variations(module_id: &str) -> Vec<String> {
    let mut variations = Vec::new();
    
    // Split module ID into address and name parts
    if let Some((addr_part, name_part)) = module_id.split_once("::") {
        // Try with and without 0x prefix
        if addr_part.starts_with("0x") {
            variations.push(format!("{}::{}", &addr_part[2..], name_part));
        } else {
            variations.push(format!("0x{}::{}", addr_part, name_part));
        }
        
        // Try lowercase and uppercase variants
        variations.push(format!("{}::{}", addr_part.to_lowercase(), name_part));
        variations.push(format!("{}::{}", addr_part.to_uppercase(), name_part));
        
        // Try with various length paddings (common issue with leading zeros)
        if let Some(addr_without_prefix) = addr_part.strip_prefix("0x") {
            // Try without leading zeros
            if let Some(non_zero_pos) = addr_without_prefix.find(|c| c != '0') {
                if non_zero_pos > 0 {
                    let trimmed = &addr_without_prefix[non_zero_pos..];
                    variations.push(format!("0x{}::{}", trimmed, name_part));
                }
            }
            
            // Try with full padding to 64 chars
            let padded = format!("{:0>64}", addr_without_prefix);
            variations.push(format!("0x{}::{}", padded, name_part));
        }
    }
    
    variations
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
                    signature: None,
                    signer_address: None,
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

pub struct Publish {
    pub signature: Option<Vec<u8>>,
    pub signer_address: Option<String>,
}

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
        let rerooted_path = match reroot_path(path) {
            Ok(p) => {
                info!("Package path reroot successful: {}", p.display());
                p
            },
            Err(e) => return Err(anyhow::anyhow!("Failed to reroot path: {}", e)),
        };

        // ตรวจสอบว่า sources directory มีอยู่จริง
        let sources_dir = rerooted_path.join("sources");
        if !sources_dir.exists() {
            return Err(anyhow::anyhow!(
                "Sources directory not found at {}. Make sure you're in the correct package directory.", 
                sources_dir.display()
            ));
        }
        info!("Found sources directory at: {}", sources_dir.display());
        
        // ตรวจสอบว่ามีไฟล์ .move ใน sources directory
        let has_move_files = std::fs::read_dir(&sources_dir)
            .map_err(|e| anyhow::anyhow!("Failed to read sources directory: {}", e))?
            .filter_map(Result::ok)
            .any(|entry| {
                entry.path().extension().map_or(false, |ext| ext == "move")
            });
        
        if !has_move_files {
            return Err(anyhow::anyhow!(
                "No .move files found in sources directory: {}", 
                sources_dir.display()
            ));
        }
        info!("Found .move source files");

        let address = address.unwrap_or_else(|| AccountAddress::from_hex_literal("0x1").unwrap());
        info!("Using address: 0x{}", address.to_hex());

        info!("Compiling Move package for blockchain deployment...");
        let start_time = std::time::Instant::now();

        // Compile package with timeout and panic handling
        let compile_result = std::panic::catch_unwind(|| {
            config.compile_package(&rerooted_path, &mut Vec::new())
        });
        
        let compiled_package = match compile_result {
            Ok(Ok(package)) => {
                info!("Package compilation successful in {:?}", start_time.elapsed());
                if package.root_compiled_units.is_empty() {
                    return Err(anyhow::anyhow!("No modules compiled. Check your Move code."));
                }
                info!("Successfully compiled {} modules", package.root_compiled_units.len());
                package
            },
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Failed to compile package: {}", e));
            },
            Err(_) => {
                return Err(anyhow::anyhow!("Compiler panicked during compilation"));
            }
        };

        if let (Some(signature), Some(signer)) = (&self.signature, &self.signer_address) {
            info!("Verifying transaction signature from signer: {}", signer);
            
            let mut hasher = Sha3_256::new();
            hasher.update(address.to_hex().as_bytes());
            hasher.update(rerooted_path.to_str().unwrap_or("").as_bytes());
            hasher.update(gas_budget.unwrap_or(3_000_000).to_le_bytes());
            
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            hasher.update(timestamp.to_le_bytes());
            
            let payload_hash = hasher.finalize();
            let payload_to_verify = payload_hash.as_slice();
            
            match verify_signature(signer, payload_to_verify, signature) {
                Ok(true) => {
                    info!("✅ Signature verification passed!");
                },
                Ok(false) => {
                    info!("❌ Signature verification failed! Continuing without verification.");
                },
                Err(e) => {
                    info!("⚠️ Signature verification error: {}. Continuing without verification.", e);
                }
            }
        }

        info!("Preparing deployment information...");
        let deployment_info = self.prepare_deployment(
            &compiled_package,
            address,
            gas_budget.unwrap_or(3_000_000),
            skip_verify,
        )?;
        
        info!("Submitting to blockchain...");
        let deployment_result = self.submit_to_blockchain(
            &compiled_package,
            &address,
            &deployment_info,
            self.signature.clone(),
            self.signer_address.clone()
        )?;
        
        info!("Deployment completed with transaction ID: {}", deployment_result.transaction_id);
        
        let mut signature_info = json!({
            "signed": false
        });
        
        if let (Some(_), Some(signer)) = (&self.signature, &self.signer_address) {
            signature_info = json!({
                "signed": true,
                "signer": signer,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

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
                "signature": signature_info,
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
        
        let address_str = format!("{:0>64}", address.to_hex());
        let address_0x = format!("0x{}", address_str);

        for unit in &package.root_compiled_units {
            let module = &unit.unit;
            let module_name = module.name().to_string();
            let module_id = ModuleId::new(address, Identifier::new(module_name.clone())?);
            
            let standard_module_id = module_id.to_string();
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
        signature: Option<Vec<u8>>,
        signer_address: Option<String>,
    ) -> anyhow::Result<DeploymentResult> {
        info!(
            "Submitting {} modules to blockchain at address: 0x{}",
            package.root_compiled_units.len(),
            address.to_hex()
        );
    
        let start = std::time::Instant::now();
    
        let modules = deployment_info["modules"].as_array().unwrap();
        let mut total_gas_used = 0;
        let mut modules_deployed = 0;
        
        let blockchain = BLOCKCHAIN_DATA.iter();
        let block_height = match blockchain.last() {
            Some(block) => block.index,
            None => 0,
        };
    
        info!("Acquiring VM_STATE lock for writing...");
        let mut vm_state = match VM_STATE.try_write() {
            Ok(state) => {
                info!("Successfully acquired VM_STATE lock");
                state
            },
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to lock VM state for writing: {}", e));
            }
        };
    
        // Generate transaction ID for this deployment
        let deploy_tx_id = format!("deploy_tx_{}", generate_random_hex(32));
        info!("Generated deployment transaction ID: {}", deploy_tx_id);
    
        // ตรวจสอบจำนวนโมดูลที่จะ deploy
        info!("Found {} modules to deploy", modules.len());
        if modules.is_empty() {
            return Err(anyhow::anyhow!("No modules to deploy"));
        }
        
        // List of blockchain transactions to submit
        let mut blockchain_transactions = Vec::new();
        
        // Gas fee collector address (for blockchain transaction)
        let _gas_collector = match AccountAddress::from_hex_literal(
            "0x47621776628ba3a5b9baaab38e61f4c98e893e124204bc4dad52e702e2b24ea1") {
            Ok(addr) => addr,
            Err(_) => *address // Fallback to module address if gas collector is invalid
        };
    
        for (idx, module_json) in modules.iter().enumerate() {
            let module_name = module_json["name"].as_str().unwrap_or("unknown");
            info!("Processing module {}/{}: {}", idx + 1, modules.len(), module_name);
            
            // ค้นหา compiled unit ที่ตรงกับโมดูลนี้
            let bytecode = match package.root_compiled_units.iter().find(|unit| unit.unit.name().to_string() == module_name) {
                Some(unit) => {
                    info!("Found compiled unit for module '{}', serializing bytecode", module_name);
                    let bytecode = unit.unit.serialize(None);
                    info!("Bytecode size: {} bytes", bytecode.len());
                    bytecode
                },
                None => {
                    info!("Warning: Could not find compiled unit for module '{}', using placeholder", module_name);
                    let size_bytes = module_json["size_bytes"].as_u64().unwrap_or(1024) as usize;
                    vec![0u8; size_bytes]
                }
            };
            
            let size_bytes = bytecode.len() as u64;
            
            let public_funcs = module_json["public_functions"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>();
            
            info!("Module {} has {} public functions", module_name, public_funcs.len());
            
            let module_id = format!("0x{}::{}", address.to_hex(), module_name);
            
            // Create VMModule for in-memory registry
            let vm_module = VMModule::new(
                *address,
                module_name.to_string(),
                bytecode.clone(),
                public_funcs.clone(),
                block_height,
            );
            
            let priority_boost = size_bytes / 100;
            let gas_used = mona_types::gas::calculate_gas_fee(Some(priority_boost));
            total_gas_used += gas_used;
            
            // Create actual blockchain transaction for this module deployment
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            // Prepare transaction data - VM module deployment format
            let mut tx_data = Vec::new();
            // Format: "VM_MODULE:<module_name>:<size>:<hash>"
            let module_hash = {
                let mut hasher = Sha3_256::new();
                hasher.update(&bytecode);
                hex::encode(hasher.finalize())
            };
            let data_str = format!("VM_MODULE:{}:{}:{}", module_name, bytecode.len(), module_hash);
            tx_data.extend_from_slice(data_str.as_bytes());
            
            // Create blockchain transaction for this module
            let blockchain_tx = mona_blockchain::block::Transaction {
                transaction_id: format!("{}_{}", deploy_tx_id, module_name),
                sender: (*address).into(),
                receiver: (*address).into(), // Send to self for module deployment
                amount: 0, // No tokens transferred for module deployment
                timestamp,
                gas_fee: gas_used,
                signature: signature.clone().unwrap_or_default(),
                data: Some(tx_data), // Include module data
            };
            
            blockchain_transactions.push(blockchain_tx);
            
            // บันทึกโมดูลลงฐานข้อมูล
            let module_abi = serde_json::to_string(module_json).unwrap_or_else(|_| "{}".to_string());
            info!("Storing module {} in database", module_id);
            match store_module(
                &module_id,
                &format!("0x{}", address.to_hex()),
                module_name,
                &bytecode,
                &module_abi,
                &deploy_tx_id,
                block_height
            ) {
                Ok(_) => {
                    info!("Module {} successfully stored in database", module_id);
                },
                Err(e) => {
                    info!("Failed to store module {} in database: {:?}", module_id, e);
                }
            }
            
            info!("Registering module {} in VM_STATE", module_id);
            
            // Make absolutely sure module is registered in a way that can be found later
            // Make sure to register with both formats (with and without 0x prefix)
            let _alt_module_id = if module_id.starts_with("0x") {
                module_id[2..].to_string()
            } else {
                format!("0x{}", module_id)
            };
            
            // Register with original module ID
            vm_state.register_module(vm_module.clone());
            
            // Also register under full address with padding for maximum compatibility
            let padded_addr = format!("{:0>64}", address.to_hex());
            let full_module_id = format!("0x{}::{}", padded_addr, module_name);
            
            log::info!("Also registering module under ID: {}", full_module_id);
            let mut vm_module_copy = vm_module.clone();
            vm_module_copy.module_id = full_module_id;
            vm_state.register_module(vm_module_copy);
            
            modules_deployed += 1;
            
            info!(
                "Module '{}' deployed and registered - size: {} bytes, gas used: {} ({})",
                module_name,
                size_bytes,
                gas_used,
                mona_types::gas::format_gas_fee_display(gas_used)
            );
        }
    
        // บันทึกข้อมูลลายเซ็น
        if let (Some(sig), Some(signer)) = (&signature, &signer_address) {
            info!(
                "Deployment transaction signed by {} (signature: {} bytes)",
                signer,
                sig.len()
            );
            
            vm_state.last_signature = Some(hex::encode(sig));
            vm_state.last_signer = Some(signer.clone());
        } else {
            info!("Deployment transaction is not signed or signature verification failed");
        }
    
        let execution_time = start.elapsed().as_millis();
        
        // อัพเดตสถิติใน VM_STATE
        vm_state.execution_count += 1;
        vm_state.last_execution = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Drop VM_STATE lock
        drop(vm_state);
        info!("Released VM_STATE lock");

        // Submit transactions to blockchain queue
        info!("Submitting {} module deployment transactions to blockchain", blockchain_transactions.len());
        for tx in blockchain_transactions {
            let tx_id = tx.transaction_id.clone(); // Clone the ID before tx is moved
            if let Err(e) = submit_transaction(tx) {
                warn!("Failed to submit module transaction to blockchain: {}", e);
            } else {
                info!("Module transaction submitted to blockchain queue: {}", tx_id);
            }
        }
    
        let result = DeploymentResult {
            transaction_id: deploy_tx_id,
            status: "COMMITTED".to_string(),
            gas_used: total_gas_used,
            execution_time_ms: execution_time as u64,
            block_height: block_height as u64,
            modules_deployed,
        };
    
        info!(
            "Blockchain deployment completed in {}ms. Transaction ID: {}, gas used: {}, modules deployed: {}",
            execution_time,
            result.transaction_id,
            result.gas_used,
            modules_deployed
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
    modules_deployed: usize,  // Track how many modules were deployed
}

fn estimate_gas_for_module(bytecode: &[u8], gas_budget: u64) -> u64 {
    let base_cost = mona_types::gas::BASE_GAS_FEE;
    let size_cost = bytecode.len() as u64 * 10;
    let priority_boost = bytecode.len() as u64 / 100;
    let network_stats = mona_types::gas::get_network_stats();
    let congestion_factor = if network_stats.pending_transactions > 0 {
        (1.0 + (network_stats.pending_transactions as f64 / 10.0).ln_1p()) 
            * mona_types::gas::CONGESTION_MULTIPLIER
    } else {
        1.0
    };
    let estimate = base_cost + (size_cost as f64 * congestion_factor) as u64;
    let estimate = estimate + priority_boost;
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
            if func_def.visibility == move_binary_format::file_format::Visibility::Public {
                let func_handle = compiled_module.function_handle_at(func_def.function);
                let func_name = compiled_module.identifier_at(func_handle.name).to_string();
                let complete_func_path = format!("{}::{}", full_module_id, module_name);
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

// Utility function to clone a VMModule for multiple registrations
impl Clone for VMModule {
    fn clone(&self) -> Self {
        Self {
            module_id: self.module_id.clone(),
            address: self.address,
            name: self.name.clone(),
            bytecode: self.bytecode.clone(),
            public_functions: self.public_functions.clone(),
            deploy_block_height: self.deploy_block_height,
        }
    }
}