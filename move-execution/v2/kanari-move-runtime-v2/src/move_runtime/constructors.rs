// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MoveRuntime {
    pub(super) fn read_vm(&self) -> RwLockReadGuard<'_, MoveVM> {
        self.vm
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn write_vm(&self) -> RwLockWriteGuard<'_, MoveVM> {
        self.vm
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Create a new runtime with custom native functions and default storage.
    pub(crate) fn new_with_natives(natives: Vec<NativeFunctionTable>) -> Result<Self> {
        let state = if cfg!(miri) {
            MoveVMState::new_in_memory()?
        } else {
            MoveVMState::open_default()?
        };
        Self::new_internal(natives, state, None)
    }

    /// Create a new runtime with custom native functions using in-memory storage.
    pub fn new_with_natives_in_memory(natives: Vec<NativeFunctionTable>) -> Result<Self> {
        let state = MoveVMState::new_in_memory()?;
        Self::new_internal(natives, state, None)
    }

    /// Create a new runtime with custom native functions and a shared persistent store.
    pub(crate) fn new_with_natives_and_store(
        natives: Vec<NativeFunctionTable>,
        store: Arc<PersistentStore>,
    ) -> Result<Self> {
        let state = MoveVMState::new(store.clone());
        Self::new_internal(natives, state, Some(store))
    }

    fn new_internal(
        natives: Vec<NativeFunctionTable>,
        state: MoveVMState,
        shared_store: Option<Arc<PersistentStore>>,
    ) -> Result<Self> {
        let all_natives: NativeFunctionTable = natives
            .into_iter()
            .flat_map(|table| table.into_iter())
            .collect();

        let vm = MoveVM::new(all_natives.clone()).require("VM init error")?;

        let object_storage: Arc<dyn ObjectStore> = match shared_store {
            Some(store) if !cfg!(miri) => match ObjectStorage::boxed_with_store(store) {
                Ok(store) => Arc::from(store),
                Err(e) => {
                    log::warn!("[RUNTIME] shared object store load failed: {}", e);
                    Arc::from(ObjectStorage::boxed_inmemory())
                }
            },
            _ if cfg!(miri) => Arc::from(ObjectStorage::boxed_inmemory()),
            _ => match ObjectStorage::boxed_with_store(state.store()) {
                Ok(store) => Arc::from(store),
                Err(e) => {
                    log::warn!("[RUNTIME] shared object store load failed: {}", e);
                    Arc::from(ObjectStorage::boxed_inmemory())
                }
            },
        };
        let resolver = KanariMoveResolver::without_trace(state.clone(), object_storage.clone());

        let published_modules: HashSet<ModuleId> = state
            .get_all_module_ids()
            .context("Failed to load persistent Move module index")?
            .into_iter()
            .collect();

        Ok(MoveRuntime {
            vm: Arc::new(RwLock::new(vm)),
            all_natives: Arc::new(all_natives),
            resolver,
            state,
            published_modules: Arc::new(RwLock::new(published_modules)),
            object_storage,
            type_tag_cache: Arc::new(RwLock::new(HashMap::new())),
            module_publish_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Create a new runtime with Kanari system natives and default storage.
    pub fn new_with_kanari_natives() -> Result<Self> {
        let natives = Self::get_kanari_natives_list();
        let runtime = Self::new_with_natives(natives)?;
        if !cfg!(miri) {
            runtime.load_system_modules()?;
        }
        Ok(runtime)
    }

    /// Create a new runtime with Kanari system natives using in-memory storage.
    pub fn new_with_kanari_natives_in_memory() -> Result<Self> {
        let natives = Self::get_kanari_natives_list();
        let runtime = Self::new_with_natives_in_memory(natives)?;
        runtime.load_embedded_system_modules()?;
        Ok(runtime)
    }

    /// Create a new runtime with Kanari system natives and a shared persistent store.
    pub fn new_with_kanari_natives_and_store(store: Arc<PersistentStore>) -> Result<Self> {
        Self::new_with_kanari_natives_and_store_framework_source(store, false)
    }

    /// Create a runtime sharing `store` while loading the embedded frameworks.
    /// Used by in-memory engines so their framework dependency set is stable
    /// and independent of whatever build artifacts happen to be on disk.
    pub fn new_with_kanari_natives_and_store_with_embedded_frameworks(
        store: Arc<PersistentStore>,
    ) -> Result<Self> {
        Self::new_with_kanari_natives_and_store_framework_source(store, true)
    }

    fn new_with_kanari_natives_and_store_framework_source(
        store: Arc<PersistentStore>,
        use_embedded: bool,
    ) -> Result<Self> {
        let natives = Self::get_kanari_natives_list();
        let runtime = Self::new_with_natives_and_store(natives, store)?;
        if use_embedded {
            runtime.load_embedded_system_modules()?;
        } else {
            runtime.load_system_modules()?;
        }
        Ok(runtime)
    }

    fn get_kanari_natives_list() -> Vec<NativeFunctionTable> {
        let sys_addr = KanariAddress::kanari_system_account_address();
        vec![
            move_stdlib_natives::all_natives(
                KanariAddress::std_account_address(),
                move_stdlib_natives::GasParameters::zeros(),
            ),
            // Node execution prices system natives with the production
            // schedule; tests/dev tooling (move_cli Test, unit tests) keep
            // `zeros()` so assertions stay gas-agnostic.
            kanari_system_natives::all_natives(
                sys_addr,
                kanari_system_natives::GasParameters::zeros(),
            ),
        ]
    }

    /// Spawn a worker runtime that shares the same state and native table.
    pub fn spawn_worker(&self) -> Result<Self> {
        let vm = MoveVM::new(self.all_natives.as_ref().clone()).require("Worker VM init error")?;

        Ok(MoveRuntime {
            vm: Arc::new(RwLock::new(vm)),
            all_natives: self.all_natives.clone(),
            resolver: self.resolver.clone(),
            state: self.state.clone(),
            published_modules: self.published_modules.clone(),
            object_storage: self.object_storage.clone(),
            type_tag_cache: self.type_tag_cache.clone(),
            module_publish_lock: self.module_publish_lock.clone(),
        })
    }

    /// Spawn an isolated worker with its own VM, resolver, and object cache.
    pub fn spawn_isolated_worker(&self) -> Result<Self> {
        let vm = MoveVM::new(self.all_natives.as_ref().clone())
            .require("Isolated worker VM init error")?;

        let object_storage: Arc<dyn ObjectStore> =
            match ObjectStorage::boxed_with_store(self.state.store()) {
                Ok(store) => Arc::from(store),
                Err(e) => {
                    log::warn!("[RUNTIME] isolated object store load failed: {}", e);
                    Arc::from(ObjectStorage::boxed_inmemory())
                }
            };

        let resolver =
            KanariMoveResolver::without_trace(self.state.clone(), object_storage.clone());

        let published_modules = self
            .published_modules
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();

        let runtime = MoveRuntime {
            vm: Arc::new(RwLock::new(vm)),
            all_natives: self.all_natives.clone(),
            resolver,
            state: self.state.clone(),
            published_modules: Arc::new(RwLock::new(published_modules)),
            object_storage,
            type_tag_cache: Arc::new(RwLock::new(HashMap::new())),
            module_publish_lock: Arc::new(Mutex::new(())),
        };

        runtime.preload_system_modules_into_vm()?;
        Ok(runtime)
    }

    /// Rebuild the VM instance so cached module state is refreshed.
    pub fn reload_vm_cache(&self) -> Result<()> {
        let new_vm =
            MoveVM::new(self.all_natives.as_ref().clone()).require("Failed to reload MoveVM")?;

        // Replace the VM instance to clear internal caches.
        *self.write_vm() = new_vm;

        // Preload published modules so follow-up executions can resolve dependencies immediately.
        self.preload_system_modules_into_vm()?;

        log::debug!("[RUNTIME] MoveVM cache cleared and reloaded");
        Ok(())
    }

    /// Refresh module identity and VM caches after canonical state has committed.
    pub fn refresh_committed_modules(&self) -> Result<()> {
        let module_ids = self.state.get_all_module_ids()?;
        *self
            .published_modules
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = module_ids.into_iter().collect();
        self.reload_vm_cache()
    }

    /// Clear the in-memory object cache.
    pub fn clear_object_cache(&self) -> Result<()> {
        self.object_storage
            .clear()
            .require("Failed to clear runtime object cache")
    }

    /// Preload system modules into the VM cache to ensure dependencies are available
    fn preload_system_modules_into_vm(&self) -> Result<()> {
        let vm_guard = self.read_vm();
        let session = self.create_session_with_storage_ext(&vm_guard);

        // Read the published module set from the runtime index.
        let module_ids: Vec<ModuleId> = self
            .published_modules
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect();

        // Deserialize published modules so the fresh VM repopulates its caches.
        for module_id in module_ids {
            if let Some(module_bytes) = self.state.try_get_module(&module_id)?
                && CompiledModule::deserialize_with_defaults(&module_bytes).is_ok()
            {
                log::debug!("[RUNTIME] Preloaded module into cache: {}", module_id);
            }
        }

        drop(session);
        drop(vm_guard);

        Ok(())
    }

    /// Apply a Move changeset to persistent storage and update the published module index.
    pub(crate) fn apply_move_changeset(
        &self,
        move_cs: move_core_types::effects::ChangeSet,
    ) -> Result<()> {
        let store = self.state.store();
        let transaction_guard = store.transaction_guard();
        let mut updates = Vec::<(Vec<u8>, Vec<u8>)>::new();
        let mut deletes = Vec::<Vec<u8>>::new();
        let mut module_index = store
            .load::<Vec<String>>(b"module_index")?
            .unwrap_or_default()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut published_module_updates = Vec::new();
        let mut published_module_deletes = Vec::new();
        let mut refreshed_objects = Vec::<StoredObject>::new();

        for (addr, account_changes) in move_cs.accounts() {
            for (module_name, op) in account_changes.modules() {
                let module_id = ModuleId::new(*addr, Identifier::new(module_name.as_str())?);
                let key =
                    crate::common::keys::module_key(module_id.address(), module_id.name().as_str());
                match op {
                    move_core_types::effects::Op::New(bytes)
                    | move_core_types::effects::Op::Modify(bytes) => {
                        updates.push((key.as_bytes().to_vec(), bcs::to_bytes(bytes)?));
                        module_index.insert(key);
                        published_module_updates.push(module_id);
                    }
                    move_core_types::effects::Op::Delete => {
                        deletes.push(key.as_bytes().to_vec());
                        module_index.remove(&key);
                        published_module_deletes.push(module_id);
                    }
                }
            }
            for (struct_tag, op) in account_changes.resources() {
                let key = crate::common::keys::resource_key(addr, struct_tag);
                match op {
                    move_core_types::effects::Op::New(bytes)
                    | move_core_types::effects::Op::Modify(bytes) => {
                        updates.push((key.as_bytes().to_vec(), bcs::to_bytes(bytes)?));

                        // Objects live under their own id, so a resource write is
                        // also an object-data write. Keep `object:{id}` in step for
                        // every type (not just `coin::Coin`) or mutated objects read
                        // stale bytes on the next call.
                        let object_key = crate::common::keys::object_key(&addr.to_hex_literal());
                        let stored = store.load::<StoredObject>(&object_key)?;
                        if let Some(mut object) = stored {
                            object.data = bytes.to_vec();
                            updates.push((object_key, bcs::to_bytes(&object)?));
                            refreshed_objects.push(object);
                        }
                    }
                    move_core_types::effects::Op::Delete => {
                        deletes.push(key.into_bytes());
                    }
                }
            }
        }

        updates.push((
            b"module_index".to_vec(),
            bcs::to_bytes(&module_index.into_iter().collect::<Vec<_>>())?,
        ));
        store.apply_raw_changes(&updates, &deletes)?;

        let mut modules = self
            .published_modules
            .write()
            .unwrap_or_else(|p| p.into_inner());
        modules.extend(published_module_updates);
        for module_id in published_module_deletes {
            modules.remove(&module_id);
        }
        drop(transaction_guard);

        // `object_storage` caches what `store` now holds; refresh only the
        // records rewritten above so later lookups see the mutated bytes.
        for object in refreshed_objects {
            self.object_storage.refresh_cached_object(object);
        }
        Ok(())
    }

    pub(super) fn load_system_modules(&self) -> Result<()> {
        self.load_system_modules_with_source(false)
    }

    pub(super) fn load_embedded_system_modules(&self) -> Result<()> {
        self.load_system_modules_with_source(true)
    }

    pub(super) fn load_system_modules_with_source(&self, use_embedded: bool) -> Result<()> {
        self.load_move_stdlib(use_embedded)?;
        self.load_kanari_system(use_embedded)?;
        Ok(())
    }
}
