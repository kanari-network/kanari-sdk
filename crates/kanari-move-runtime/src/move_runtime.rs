// This file contains the MoveRuntime wrapper implementation.
// It utilizes MoveVM and InMemoryStorage for executing functions and publishing modules.
// Enhanced with native function support, gas metering, and session management.

use anyhow::Result;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use move_core_types::effects::Op as MoveOp;
use move_core_types::identifier::IdentStr;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_runtime::native_functions::NativeFunctionTable;
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

use crate::gas::{GasMeter, GasOperation};
use kanari_types::address::Address as KanariAddress;

use crate::changeset::ChangeSet;
use crate::move_vm_state::MoveVMState;
use crate::object_storage::ObjectStorage;
use kanari_types::tx_context::TxContextRecord;

use std::collections::HashSet;

/// Enhanced runtime wrapper around `move-vm` for executing functions, publishing modules,
/// and running scripts. Supports custom native functions, gas metering, and atomic sessions.
pub struct MoveRuntime {
    pub(crate) vm: MoveVM,
    pub(crate) storage: InMemoryStorage,
    pub(crate) state: MoveVMState,
    /// Whether to use gas metering in VM sessions
    pub(crate) enable_gas_metering: bool,
    /// Index of published modules for faster listing
    pub(crate) published_modules: HashSet<ModuleId>,
    /// Persistent object storage for transferred objects
    pub(crate) object_storage: ObjectStorage,
}

impl MoveRuntime {
    /// Open the runtime using the default persistent DB path (see README).
    /// Initializes VM without custom natives (for basic usage).
    pub fn new() -> Result<Self> {
        Self::new_with_natives(vec![], false)
    }

    /// Create a new MoveRuntime with custom native functions.
    ///
    /// # Arguments
    /// * `natives` - Native function tables for custom functions (e.g., crypto operations)
    /// * `enable_gas_metering` - Whether to enable gas metering in VM sessions
    ///
    /// # Example
    /// ```ignore
    /// use kanari_crypto::move_natives;
    /// use move_core_types::account_address::AccountAddress;
    ///
    /// let system_addr = AccountAddress::from_hex_literal("0x2").unwrap();
    /// let natives = move_natives::all_natives(system_addr);
    /// let runtime = MoveRuntime::new_with_natives(vec![natives], true)?;
    /// ```
    pub fn new_with_natives(
        natives: Vec<NativeFunctionTable>,
        enable_gas_metering: bool,
    ) -> Result<Self> {
        let state = MoveVMState::open_default()?;
        let mut storage = InMemoryStorage::new();
        state.load_into_storage(&mut storage)?;

        // Flatten all native tables into a single iterator
        let all_natives: Vec<_> = natives
            .into_iter()
            .flat_map(|table| table.into_iter())
            .collect();

        // Initialize VM with custom natives
        let vm = MoveVM::new(all_natives)
            .map_err(|e| anyhow::anyhow!(format!("VM init error: {:?}", e)))?;

        Ok(MoveRuntime {
            vm,
            storage,
            state,
            enable_gas_metering,
            published_modules: HashSet::new(),
            object_storage: ObjectStorage::new(),
        })
    }

    /// Create runtime with Kanari system natives (crypto + stdlib + object + tx_context)
    /// Also loads pre-compiled Kanari system modules (0x2::*)
    pub fn new_with_kanari_natives() -> Result<Self> {
        // Standard library natives at 0x1
        let std_addr = AccountAddress::from_hex_literal("0x1")?;
        let std_natives =
            move_stdlib_natives::all_natives(std_addr, move_stdlib_natives::GasParameters::zeros());

        // Kanari crypto natives at 0x2
        let system_addr = AccountAddress::from_hex_literal("0x2")?;
        let crypto_natives = kanari_crypto::move_natives::all_natives(system_addr);

        // Transfer natives at 0x2 (same address as kanari_system)
        let transfer_natives = crate::transfer_natives::all_natives(system_addr);

        // Create runtime with natives
        let mut runtime =
            Self::new_with_natives(vec![std_natives, crypto_natives, transfer_natives], true)?;

        // Load pre-compiled Kanari system modules
        runtime.load_system_modules()?;

        Ok(runtime)
    }

    /// Load pre-compiled Kanari system modules from the framework package
    /// This publishes all 0x2::* modules (transfer, coin, balance, etc.) to storage
    fn load_system_modules(&mut self) -> Result<()> {
        // First, load move-stdlib modules (0x1::*)
        self.load_move_stdlib()?;

        // Then load Kanari system modules (0x2::*)
        self.load_kanari_system()?;

        Ok(())
    }

    /// Load move-stdlib modules (0x1::*)
    fn load_move_stdlib(&mut self) -> Result<()> {
        // Determine stdlib path. Allow override via MOVE_STDLIB_PATH env var.
        let modules_dir = if let Ok(path_str) = std::env::var("MOVE_STDLIB_PATH") {
            std::path::PathBuf::from(path_str)
        } else {
            // Try two candidate locations to avoid duplicated `crates/crates` when cwd is already the `crates` folder.
            let cwd = std::env::current_dir().unwrap_or_default();
            let candidate1 = cwd
                .join("crates")
                .join("kanari-frameworks")
                .join("packages")
                .join("move-stdlib")
                .join("build")
                .join("MoveStdlib")
                .join("bytecode_modules");

            if candidate1.exists() {
                candidate1
            } else if let Some(parent) = cwd.parent() {
                let candidate2 = parent
                    .join("crates")
                    .join("kanari-frameworks")
                    .join("packages")
                    .join("move-stdlib")
                    .join("build")
                    .join("MoveStdlib")
                    .join("bytecode_modules");
                candidate2
            } else {
                candidate1
            }
        };

        println!("✓ Looking for Move stdlib modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            eprintln!(
                "Warning: Move stdlib modules not found at {:?}",
                modules_dir
            );
            eprintln!("Standard library will not be pre-loaded.");
            return Ok(());
        }

        // Load stdlib modules in dependency order
        let std_addr = AccountAddress::from_hex_literal("0x1")?;
        let module_order = vec![
            "vector.mv",
            "error.mv",
            "address.mv",
            "signer.mv",
            "option.mv",
            "fixed_point32.mv",
            "ascii.mv",
            "string.mv",
            "hash.mv",
            "bcs.mv",
            "bit_vector.mv",
            "type_name.mv",
        ];

        let mut count = 0;
        for module_file in module_order {
            let module_path = modules_dir.join(module_file);
            if let Ok(module_bytes) = std::fs::read(&module_path) {
                match self.publish_module(module_bytes, std_addr, None) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        // Silently skip already loaded modules
                        if !e.to_string().contains("already exists") {
                            eprintln!("Warning: Failed to load {}: {}", module_file, e);
                        }
                    }
                }
            }
        }

        println!("✓ Loaded {} move-stdlib modules (0x1::*)", count);
        Ok(())
    }

    /// Load Kanari system modules (0x2::*)
    fn load_kanari_system(&mut self) -> Result<()> {
        // Path to pre-compiled Kanari system modules
        // Determine kanari system framework path (allow override via KANARI_FRAMEWORK_PATH).
        let modules_dir = if let Ok(path_str) = std::env::var("KANARI_FRAMEWORK_PATH") {
            std::path::PathBuf::from(path_str)
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            let candidate1 = cwd
                .join("crates")
                .join("kanari-frameworks")
                .join("packages")
                .join("kanari-system")
                .join("build")
                .join("KanariSystem")
                .join("bytecode_modules");

            if candidate1.exists() {
                candidate1
            } else if let Some(parent) = cwd.parent() {
                parent
                    .join("crates")
                    .join("kanari-frameworks")
                    .join("packages")
                    .join("kanari-system")
                    .join("build")
                    .join("KanariSystem")
                    .join("bytecode_modules")
            } else {
                candidate1
            }
        };

        println!("✓ Looking for Kanari system modules at: {:?}", modules_dir);

        if !modules_dir.exists() {
            eprintln!(
                "Warning: Kanari system modules not found at {:?}",
                modules_dir
            );
            eprintln!(
                "System modules will not be pre-loaded. You may need to publish them manually."
            );
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  cd crates/kanari-frameworks");
            eprintln!("  sui move build -p packages/kanari-system");
            return Ok(());
        }

        // List of system modules to load in dependency order
        // Load system modules in dependency order. Note: some modules (coin) depend on transfer,
        // so transfer must be published before coin.
        let module_files = vec![
            "tx_context.mv",
            "object.mv",
            "url.mv",
            "balance.mv",
            "transfer.mv",
            // Deny-list must be published before coin which references it
            "deny_list.mv",
            "coin.mv",
            "kanari.mv",
            // Crypto modules (these are wrappers for native functions)
            "ecdsa_k1.mv",
            "ecdsa_r1.mv",
            "ed25519.mv",
        ];

        let system_addr = AccountAddress::from_hex_literal("0x2")?;
        let mut count = 0;

        for module_file in module_files {
            let module_path = modules_dir.join(module_file);

            if let Ok(module_bytes) = std::fs::read(&module_path) {
                // Publish module silently (no gas accounting for system modules)
                match self.publish_module(module_bytes.clone(), system_addr, None) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        // Silently skip already loaded modules
                        if !e.to_string().contains("already exists") {
                            eprintln!("Warning: Failed to load {}: {}", module_file, e);
                        }
                    }
                }
            } else {
                eprintln!("Warning: Module file not found: {:?}", module_path);
            }
        }

        println!("✓ Loaded {} kanari-system modules (0x2::*)", count);
        Ok(())
    }

    /// Publish a module (bytes) with the given sender address.
    /// Returns ChangeSet containing the module addition and any resource changes from Move VM.
    pub fn publish_module(
        &mut self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        // Optional gas tuple: (gas_limit, gas_price). If `Some`, runtime will
        // include gas accounting (debit sender, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
    ) -> Result<ChangeSet> {
        // Deserialize to get module ID early
        let compiled = CompiledModule::deserialize_with_defaults(&module_bytes)
            .map_err(|e| anyhow::anyhow!(format!("deserialize error: {:?}", e)))?;
        let module_id = compiled.self_id();

        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

        session
            .publish_module(module_bytes.clone(), sender, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("publish error: {:?}", e)))?;

        let (res, new_storage) = session.finish();
        let (move_changeset, events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        // Apply changeset - if module exists, this will fail with "already exists"
        // In that case, we'll handle it as an upgrade
        let mut storage = new_storage;
        let apply_result = storage.apply(move_changeset.clone());

        match apply_result {
            Ok(_) => {
                // New module published successfully
                self.storage = storage.clone();
                self.published_modules.insert(module_id.clone());
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                if err_msg.contains("already exists") {
                    // Module upgrade - use publish_or_overwrite instead of apply
                    self.storage
                        .publish_or_overwrite_module(module_id.clone(), module_bytes.clone());
                    self.published_modules.insert(module_id.clone());
                } else {
                    return Err(anyhow::anyhow!(format!("apply error: {:?}", e)));
                }
            }
        }

        // persist module bytes to DB (overwrite if exists for upgrades)
        self.state.save_module(&module_id, &module_bytes)?;

        // Create ChangeSet from Move VM changeset
        let mut cs = ChangeSet::new();
        cs.publish_module(sender, module_id.name().to_string());

        // If caller provided gas info, include gas accounting in the ChangeSet.
        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::PublishModule {
                module_size: module_bytes.len(),
            };
            self.apply_gas_info(&mut cs, Some(sender), gas_limit, gas_price, gas_op)?;
        }

        // Parse Move VM changeset and events
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        // Auto-call init() function if it exists. Move VM won't call init() automatically,
        // so call it here and merge its returned ChangeSet into the publish ChangeSet.
        if self.module_has_init_function(&compiled) {
            println!("[PUBLISH] ✓ Module has init() function - calling init() now");
            match self.auto_call_init(&module_id, sender) {
                Ok(init_cs) => {
                    // Merge init() changes into publish changeset
                    cs.merge(init_cs);
                    println!("[PUBLISH] ✓ init() executed and merged into ChangeSet");
                }
                Err(e) => {
                    eprintln!("[PUBLISH] ✗ init() call failed: {}", e);
                }
            }
        }

        Ok(cs)
    }

    /// Publish a bundle of modules atomically. This helps resolving inter-module dependencies.
    pub fn publish_module_bundle(
        &mut self,
        modules: Vec<Vec<u8>>,
        sender: AccountAddress,
    ) -> Result<()> {
        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

        session
            .publish_module_bundle(modules.clone(), sender, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("publish bundle error: {:?}", e)))?;

        let (res, new_storage) = session.finish();
        let (changeset, _events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        let mut storage = new_storage;
        storage
            .apply(changeset)
            .map_err(|e| anyhow::anyhow!(format!("apply error: {:?}", e)))?;

        // update runtime storage
        self.storage = storage.clone();

        // persist each compiled module to DB
        for module_bytes in modules.into_iter() {
            let compiled = CompiledModule::deserialize_with_defaults(&module_bytes)
                .map_err(|e| anyhow::anyhow!(format!("deserialize error: {:?}", e)))?;
            let module_id = compiled.self_id();
            self.state.save_module(&module_id, &module_bytes)?;
        }

        Ok(())
    }

    /// Attempt to publish modules in an order that satisfies dependencies by retrying
    /// individual publishes. Each module is published with its declared `self_id().address()` as sender.
    pub fn publish_modules_ordered(&mut self, modules: Vec<Vec<u8>>) -> Result<()> {
        use std::collections::VecDeque;
        let mut queue: VecDeque<Vec<u8>> = VecDeque::from(modules);
        let mut made_progress = true;
        let mut last_err: Option<anyhow::Error> = None;

        while !queue.is_empty() && made_progress {
            made_progress = false;
            let len = queue.len();
            for _ in 0..len {
                let bytes = queue.pop_front().unwrap();
                // try to deserialize to get module address
                match CompiledModule::deserialize_with_defaults(&bytes) {
                    Ok(compiled) => {
                        let mod_id = compiled.self_id();
                        let sender = AccountAddress::from_hex_literal(&format!(
                            "0x{}",
                            mod_id.address().short_str_lossless()
                        ))
                        .unwrap_or(mod_id.address().clone());
                        let res = self.publish_module(bytes.clone(), sender, None);
                        match res {
                            Ok(_changeset) => made_progress = true,
                            Err(e) => {
                                last_err = Some(e);
                                // push back for another attempt later
                                queue.push_back(bytes);
                            }
                        }
                    }
                    Err(e) => {
                        last_err = Some(anyhow::anyhow!(format!("deserialize error: {:?}", e)));
                        // cannot determine sender, give up on this module
                    }
                }
            }
        }

        if !queue.is_empty() {
            return Err(last_err.unwrap_or_else(|| {
                anyhow::anyhow!("failed to publish modules due to unresolved dependencies")
            }));
        }
        Ok(())
    }

    /// Execute an entry function. `type_args` are Move `TypeTag`s and `args` are serialized
    /// arguments as Vec<u8> (Move simple-serialized values).
    /// Returns ChangeSet containing all state changes from Move VM execution.
    pub fn execute_entry_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        // Optional sender address. If provided along with `gas_info`, runtime will debit this sender.
        sender: Option<AccountAddress>,
        // Optional gas tuple: (gas_limit, gas_price). If provided, runtime will
        // include gas accounting (debit sender if available, credit DAO) in the returned ChangeSet.
        gas_info: Option<(u64, u64)>,
    ) -> Result<ChangeSet> {
        let storage_clone = self.storage.clone();
        let mut session = self.vm.new_session(storage_clone);
        let mut gas = UnmeteredGasMeter;

        // convert type tags to VM runtime types
        let mut ty_args_loaded = vec![];
        for tag in type_args.iter() {
            let ty = session
                .load_type(tag)
                .map_err(|e| anyhow::anyhow!(format!("load type error: {:?}", e)))?;
            ty_args_loaded.push(ty);
        }

        let ident = IdentStr::new(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Auto-inject TxContext if function expects it as last parameter
        let mut final_args = args.clone();

        // Create TxContext struct using canonical Kanari type and serialize with BCS
        let sender_addr = sender.unwrap_or(AccountAddress::ZERO);
        let tx_hash = vec![0u8; 32]; // Placeholder
        let epoch = 0u64;
        let epoch_timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let ids_created = 0u64;

        let tx_ctx = TxContextRecord::from_address(
            sender_addr,
            tx_hash.clone(),
            epoch,
            epoch_timestamp_ms,
            ids_created,
        );

        let tx_context_bytes = bcs::to_bytes(&tx_ctx)?;

        // Add TxContext as last argument
        final_args.push(tx_context_bytes);

        session
            .execute_entry_function(module_id, ident, ty_args_loaded, final_args, &mut gas)
            .map_err(|e| anyhow::anyhow!(format!("exec error: {:?}", e)))?;

        let (res, new_storage) = session.finish();
        let (move_changeset, events) =
            res.map_err(|e| anyhow::anyhow!(format!("finish error: {:?}", e)))?;

        let mut storage = new_storage;
        storage
            .apply(move_changeset.clone())
            .map_err(|e| anyhow::anyhow!(format!("apply error: {:?}", e)))?;

        self.storage = storage;

        // Create ChangeSet from Move VM execution
        let mut cs = ChangeSet::new();

        // Parse Move VM changeset and events
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(&events, &mut cs);

        // Add transferred objects from native function tracking
        self.add_transferred_objects(&mut cs);

        // If gas accounting requested, include gas debit/credit in ChangeSet.
        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::ExecuteFunction { complexity: 1 };
            self.apply_gas_info(&mut cs, sender, gas_limit, gas_price, gas_op)?;
        }

        Ok(cs)
    }

    /// Get object by ID from ObjectStorage
    pub fn get_object(&self, object_id: &str) -> Option<crate::object_storage::StoredObject> {
        self.object_storage.get_object(object_id)
    }

    /// Get all objects owned by an address
    pub fn get_objects_by_owner(
        &self,
        owner: &AccountAddress,
    ) -> Vec<crate::object_storage::StoredObject> {
        self.object_storage.get_objects_by_owner(owner)
    }

    /// Transfer object ownership
    pub fn transfer_object_ownership(
        &mut self,
        object_id: &str,
        new_owner: AccountAddress,
    ) -> Result<()> {
        self.object_storage
            .transfer_object(object_id, new_owner)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Delete object from storage
    pub fn delete_object(&mut self, object_id: &str) -> Result<()> {
        self.object_storage
            .delete_object(object_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get object storage statistics
    pub fn get_object_stats(&self) -> (usize, usize) {
        let total_objects = self.object_storage.count();
        let total_owners = 0; // Could add owner count tracking if needed
        (total_objects, total_owners)
    }

    /// Helper to apply gas accounting to a ChangeSet. Handles sender debit + sequence increment
    /// and credits gas to DAO. `sender` may be `None` for system-level calls.
    fn apply_gas_info(
        &self,
        cs: &mut ChangeSet,
        sender: Option<AccountAddress>,
        gas_limit: u64,
        gas_price: u64,
        gas_op: GasOperation,
    ) -> Result<()> {
        let mut meter = GasMeter::new(gas_limit, gas_price);
        meter.consume(gas_op.gas_units())?;
        let gas_cost = meter.total_cost();

        if let Some(saddr) = sender {
            let sender_change = cs.get_or_create_change(saddr);
            sender_change.increment_sequence();
            sender_change.debit(gas_cost);
        }

        let dao_addr = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS)?;
        cs.collect_gas(dao_addr, gas_cost);
        cs.set_gas_used(meter.gas_used);
        Ok(())
    }

    /// Parse Move VM ChangeSet and extract state changes into Kanari ChangeSet
    /// This converts Move VM's canonical state changes into our domain model
    pub(crate) fn parse_move_changeset(
        &self,
        move_cs: &move_core_types::effects::ChangeSet,
        kanari_cs: &mut ChangeSet,
    ) {
        for (addr, account_changes) in move_cs.accounts() {
            // Process module changes
            for (module_name, op) in account_changes.modules() {
                match op {
                    MoveOp::New(_bytes) | MoveOp::Modify(_bytes) => {
                        // Module published or updated
                        kanari_cs.publish_module(*addr, module_name.to_string());
                    }
                    MoveOp::Delete => {
                        // Module deletion (rare, but possible)
                        eprintln!(
                            "Warning: Module deletion detected for {}::{}",
                            addr, module_name
                        );
                    }
                }
            }

            // Process resource changes
            for (struct_tag, op) in account_changes.resources() {
                match op {
                    MoveOp::New(bytes) | MoveOp::Modify(bytes) => {
                        // Parse resource bytes and set per-account token balances when applicable
                        if self.is_balance_resource(struct_tag) {
                            if let Some(amount) = self.extract_balance_from_bytes(bytes, struct_tag)
                            {
                                if let Some(token_type) =
                                    self.token_type_from_struct_tag(struct_tag)
                                {
                                    // Record absolute token balance for this account
                                    kanari_cs.add_token_balance_set(
                                        *addr,
                                        token_type.clone(),
                                        amount,
                                    );
                                    eprintln!(
                                        "Set token balance for {}: {} = {}",
                                        addr, token_type, amount
                                    );
                                }
                            }
                        }

                        // Parse TreasuryCap resources: track total_supply and owner
                        if self.is_treasury_resource(struct_tag) {
                            if let Some(total) = self.extract_treasury_total_from_bytes(bytes) {
                                if let Some(token_type) =
                                    self.token_type_from_struct_tag(struct_tag)
                                {
                                    kanari_cs.add_treasury(*addr, token_type.clone(), total);
                                    eprintln!(
                                        "TreasuryCap for {} created/updated at {} with supply {}",
                                        token_type, addr, total
                                    );
                                }
                            }
                        }

                        // Detect created objects with UID in first bytes (object::UID.addr)
                        // For all objects that have a UID field (standard Move object pattern)
                        // the serialized layout starts with the UID address (32 bytes).
                        let obj_id = if bytes.len() >= 32 {
                            // Try to extract ID from first 32 bytes (UID pattern)
                            let id_bytes = &bytes[0..32];
                            hex::encode(id_bytes)
                        } else {
                            // Generate unique ID from address + struct_tag + sequence
                            self.generate_object_id(addr, struct_tag, bytes)
                        };

                        let obj_type = if let Some(tt) = self.token_type_from_struct_tag(struct_tag)
                        {
                            tt
                        } else {
                            format!(
                                "0x{}::{}::{}",
                                struct_tag.address.short_str_lossless(),
                                struct_tag.module.as_str(),
                                struct_tag.name.as_str()
                            )
                        };

                        // Record created object (version 0 for new objects)
                        kanari_cs.add_created_object(
                            obj_id.clone(),
                            *addr,
                            obj_type.clone(),
                            bytes.to_vec(),
                            0,
                        );
                        eprintln!(
                            "Created object detected: id={} owner={} type={}",
                            obj_id, addr, obj_type
                        );
                    }
                    MoveOp::Delete => {
                        // Resource deletion: if Coin/Balance deleted, set token balance to 0
                        if self.is_balance_resource(struct_tag) {
                            if let Some(token_type) = self.token_type_from_struct_tag(struct_tag) {
                                kanari_cs.add_token_balance_set(*addr, token_type.clone(), 0);
                                eprintln!(
                                    "Deleted token resource for {}: {} -> balance 0",
                                    addr, token_type
                                );
                            }
                        } else {
                            eprintln!("Resource deleted for {}: {}", addr, struct_tag);
                        }
                    }
                }
            }
        }
    }

    /// Add transferred objects from native function tracking to changeset
    /// Also persists objects to ObjectStorage for later retrieval
    fn add_transferred_objects(&mut self, cs: &mut ChangeSet) {
        let transferred = crate::transfer_natives::take_transferred_objects();

        let count = transferred.len();
        println!("[DEBUG] Processing {} transferred objects", count);

        for obj in transferred {
            println!(
                "[DEBUG] Adding transferred object: id={}, type={}, owner={}, data_len={}",
                obj.object_id,
                obj.object_type,
                obj.recipient,
                obj.data.len()
            );

            // Add to created_objects in changeset (for immediate response)
            cs.add_created_object(
                obj.object_id.clone(),
                obj.recipient,
                obj.object_type.clone(),
                obj.data.clone(),
                0,
            );

            // Persist to ObjectStorage if flagged
            if obj.should_persist {
                let stored_obj = crate::object_storage::StoredObject {
                    id: obj.object_id.clone(),
                    owner: obj.recipient,
                    type_name: obj.object_type.clone(),
                    data: obj.data,
                    version: 0,
                };

                match self.object_storage.store_object(stored_obj) {
                    Ok(_) => {
                        println!(
                            "[DEBUG] ✓ Object {} persisted to ObjectStorage",
                            obj.object_id
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[DEBUG] ✗ Failed to persist object {}: {}",
                            obj.object_id, e
                        );
                    }
                }
            }
        }

        if count > 0 {
            println!("[DEBUG] Total {} objects added to changeset", count);
        }
    }

    /// Generate unique object ID from address, type, and data
    /// Uses Blake3 hash for deterministic but unique IDs
    fn generate_object_id(
        &self,
        owner: &AccountAddress,
        struct_tag: &move_core_types::language_storage::StructTag,
        data: &[u8],
    ) -> String {
        use kanari_crypto::hash_data_blake3;

        // Create unique input: owner + module_id + struct_name + data
        let mut input = Vec::new();
        input.extend_from_slice(owner.as_ref());
        input.extend_from_slice(struct_tag.address.as_ref());
        input.extend_from_slice(struct_tag.module.as_str().as_bytes());
        input.extend_from_slice(struct_tag.name.as_str().as_bytes());
        input.extend_from_slice(data);

        // Hash to get unique ID
        let hash = hash_data_blake3(&input);
        hex::encode(&hash[0..32]) // Use first 32 bytes for object ID
    }

    /// Check if struct tag represents a balance/coin resource
    fn is_balance_resource(
        &self,
        struct_tag: &move_core_types::language_storage::StructTag,
    ) -> bool {
        // Common patterns: Coin<T>, Balance<T>, Account<T>
        let name = struct_tag.name.as_str();
        name == "Coin" || name == "Balance" || name == "Account"
    }

    fn is_treasury_resource(
        &self,
        struct_tag: &move_core_types::language_storage::StructTag,
    ) -> bool {
        struct_tag.name.as_str() == "TreasuryCap"
    }

    /// Extract balance value from bytes for resources that may include UID + Balance
    fn extract_balance_from_bytes(
        &self,
        bytes: &[u8],
        _struct_tag: &move_core_types::language_storage::StructTag,
    ) -> Option<u64> {
        // For `Balance<T>` serialized alone, the bytes are just u64 little-endian.
        // For `Coin<T>` or `TreasuryCap<T>`, the layout is typically: UID (address) followed by u64.
        // We'll try both: if length >= 8, prefer the last 8 bytes as the u64 value.
        if bytes.len() >= 8 {
            let start = bytes.len() - 8;
            let balance_bytes: [u8; 8] = bytes[start..].try_into().ok()?;
            Some(u64::from_le_bytes(balance_bytes))
        } else {
            None
        }
    }

    fn extract_treasury_total_from_bytes(&self, bytes: &[u8]) -> Option<u64> {
        // TreasuryCap layout: UID (address) + total_supply: u64
        if bytes.len() >= 8 {
            let start = bytes.len() - 8;
            let supply_bytes: [u8; 8] = bytes[start..].try_into().ok()?;
            Some(u64::from_le_bytes(supply_bytes))
        } else {
            None
        }
    }

    fn token_type_from_struct_tag(
        &self,
        struct_tag: &move_core_types::language_storage::StructTag,
    ) -> Option<String> {
        use move_core_types::language_storage::TypeTag;
        if let Some(first) = struct_tag.type_params.get(0) {
            if let TypeTag::Struct(st) = first {
                let addr = st.address.short_str_lossless();
                let module = st.module.as_str().to_string();
                let name = st.name.as_str().to_string();
                return Some(format!("0x{}::{}::{}", addr, module, name));
            }
        }
        None
    }

    /// Check if compiled module has an init() function
    fn module_has_init_function(&self, compiled: &CompiledModule) -> bool {
        compiled.function_defs.iter().any(|func_def| {
            let func_handle = &compiled.function_handles[func_def.function.0 as usize];
            let func_name = &compiled.identifiers[func_handle.name.0 as usize];
            func_name.as_str() == "init"
        })
    }

    /// Auto-call init() function after module publish
    fn auto_call_init(
        &mut self,
        module_id: &ModuleId,
        sender: AccountAddress,
    ) -> Result<ChangeSet> {
        // For init() function with witness pattern:
        // - First parameter is the witness (module's one-time-witness struct)
        // - Last parameter is TxContext
        //
        // The witness struct typically:
        // - Has the same name as module (but uppercase)
        // - Has only `drop` ability
        // - Has no fields (zero-size struct)
        //
        // We serialize an empty struct for the witness (0 bytes for struct with no fields)

        let type_args = vec![];
        let witness_bytes = vec![]; // Empty struct with no fields = 0 bytes in BCS
        let args = vec![witness_bytes];

        // Execute init function (TxContext will be auto-injected by execute_entry_function)
        self.execute_entry_function(
            module_id,
            "init",
            type_args,
            args,
            Some(sender),
            None, // No gas info - will be handled by caller
        )
    }

    /// Parse Move VM events and add to Kanari ChangeSet
    /// Events provide an audit trail of all state changes
    pub(crate) fn parse_move_events(
        &self,
        events: &[move_core_types::effects::Event],
        kanari_cs: &mut ChangeSet,
    ) {
        use crate::changeset::Event;

        for event in events.iter() {
            let (key, sequence_number, type_tag, event_data) = event;
            let kanari_event = Event {
                key: key.clone(),
                sequence_number: *sequence_number,
                type_tag: format!("{}", type_tag),
                event_data: event_data.clone(),
            };
            kanari_cs.add_event(kanari_event);
        }
    }
}
