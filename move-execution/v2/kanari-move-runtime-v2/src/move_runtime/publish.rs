// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MoveRuntime {
    /// Upgrade an existing Move module with compatibility checks.
    pub fn upgrade_module(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.upgrade_module_with_context_and_persistence(
            module_bytes,
            sender,
            gas_info,
            timestamp,
            None,
            true,
        )
    }

    /// Publish a new Move module with verification and safety checks.
    pub fn publish_module(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.publish_module_with_context_and_persistence(
            module_bytes,
            sender,
            gas_info,
            timestamp,
            None,
            true,
        )
    }

    /// Bootstrap a module allowing overwrite of an existing module during genesis.
    pub fn bootstrap_module_with_context_and_persistence(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.publish_module_with_intent_and_persistence(
            module_bytes,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
            ModulePublishIntent::Bootstrap,
        )
    }

    /// Upgrade an existing Move module with explicit context and persistence control.
    pub fn upgrade_module_with_context_and_persistence(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.publish_module_with_intent_and_persistence(
            module_bytes,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
            ModulePublishIntent::Upgrade,
        )
    }

    /// Publish a new Move module with explicit context and persistence control.
    pub fn publish_module_with_context_and_persistence(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.publish_module_with_intent_and_persistence(
            module_bytes,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
            ModulePublishIntent::Publish,
        )
    }

    fn publish_module_with_intent_and_persistence(
        &self,
        module_bytes: Vec<u8>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
        intent: ModulePublishIntent,
    ) -> Result<ChangeSet> {
        // The caller provides these context fields for API compatibility. The
        // bytecode publish path has no TxContext and must not fabricate them;
        // they are forwarded to a module `init` when one runs at publish.
        let publish_guard = self
            .module_publish_lock
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let compiled = CompiledModule::deserialize_with_defaults(&module_bytes)?;
        let module_id = compiled.self_id();
        self.verify_module_publish_safety(sender, &module_id, &compiled, &module_bytes, intent)?;

        let mut cs = ChangeSet::new();
        let (move_changeset, events, vm_gas_used) = {
            // Separate Lock into a variable first to prevent it from being dropped immediately
            let vm_guard = self.read_vm();
            let mut session = self.create_session_with_storage_ext(&vm_guard);

            let provided_gas_limit = gas_info.map(|(limit, _)| limit).unwrap_or(1_000_000);
            let mut metered_gas = crate::kanari_gas_meter::KanariGasMeter::new(provided_gas_limit);

            session
                .publish_module(module_bytes.clone(), sender, &mut metered_gas)
                .require("Move VM operation failed")?;

            // Fresh publishes run the module's `init` in this same session
            // (upgrades never do). In-session execution sees the module
            // because this session published it, in every mode.
            if intent == ModulePublishIntent::Publish && Self::module_declares_init(&compiled) {
                self.execute_init_in_publish_session(
                    &mut session,
                    &module_id,
                    sender,
                    timestamp,
                    tx_hash.clone(),
                    &mut metered_gas,
                    &mut cs,
                )?;
            }

            let result = session
                .finish()
                .0
                .require("Failed to finish publish session")?;
            (result.0, result.1, metered_gas.gas_used())
        };

        let cs = self.finish_publish_session(
            cs,
            move_changeset,
            &events,
            vm_gas_used,
            sender,
            [module_id].iter(),
            module_bytes.len(),
            gas_info,
            persist_runtime_state,
        )?;
        drop(publish_guard);
        Ok(cs)
    }

    /// Publish a multi-module package with explicit context and persistence control.
    pub fn publish_package_with_context_and_persistence(
        &self,
        modules: Vec<(String, Vec<u8>)>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.publish_package_with_intent_and_persistence(
            modules,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
            ModulePublishIntent::Publish,
        )
    }

    /// Upgrade a multi-module package with compatibility checks.
    pub fn upgrade_package_with_context_and_persistence(
        &self,
        modules: Vec<(String, Vec<u8>)>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.publish_package_with_intent_and_persistence(
            modules,
            sender,
            gas_info,
            timestamp,
            tx_hash,
            persist_runtime_state,
            ModulePublishIntent::Upgrade,
        )
    }

    fn publish_package_with_intent_and_persistence(
        &self,
        modules: Vec<(String, Vec<u8>)>,
        sender: AccountAddress,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
        intent: ModulePublishIntent,
    ) -> Result<ChangeSet> {
        // Package publishing shares the same context rule as single modules.
        let publish_guard = self
            .module_publish_lock
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        let mut compiled_modules = Vec::with_capacity(modules.len());
        let mut total_module_size = 0usize;
        for (module_name, module_bytes) in &modules {
            let compiled = CompiledModule::deserialize_with_defaults(module_bytes)?;
            let module_id = compiled.self_id();
            if module_id.name().as_str() != module_name {
                anyhow::bail!(
                    "Module publish rejected: declared module name {} does not match bytecode self id {}",
                    module_name,
                    module_id.name()
                );
            }
            self.verify_module_publish_safety(sender, &module_id, &compiled, module_bytes, intent)?;
            total_module_size = total_module_size.saturating_add(module_bytes.len());
            compiled_modules.push((module_id, module_bytes.clone()));
        }

        let mut cs = ChangeSet::new();
        let (move_changeset, events, vm_gas_used) = {
            let vm_guard = self.read_vm();
            let mut session = self.create_session_with_storage_ext(&vm_guard);
            let provided_gas_limit = gas_info.map(|(limit, _)| limit).unwrap_or(1_000_000);
            let mut metered_gas = crate::kanari_gas_meter::KanariGasMeter::new(provided_gas_limit);

            for (_, module_bytes) in &compiled_modules {
                session
                    .publish_module(module_bytes.clone(), sender, &mut metered_gas)
                    .require("Move VM package publish failed")?;
            }

            // Fresh publishes run each declaring module's `init` in this
            // same session (upgrades never do).
            if intent == ModulePublishIntent::Publish {
                for (module_id, module_bytes) in &compiled_modules {
                    let compiled = CompiledModule::deserialize_with_defaults(module_bytes)?;
                    if !Self::module_declares_init(&compiled) {
                        continue;
                    }
                    let init_tx_hash = Self::package_init_tx_hash(tx_hash.as_deref(), module_id);
                    self.execute_init_in_publish_session(
                        &mut session,
                        module_id,
                        sender,
                        timestamp,
                        Some(init_tx_hash),
                        &mut metered_gas,
                        &mut cs,
                    )?;
                }
            }

            let result = session
                .finish()
                .0
                .require("Failed to finish publish package session")?;
            (result.0, result.1, metered_gas.gas_used())
        };

        let cs = self.finish_publish_session(
            cs,
            move_changeset,
            &events,
            vm_gas_used,
            sender,
            compiled_modules.iter().map(|(module_id, _)| module_id),
            total_module_size,
            gas_info,
            persist_runtime_state,
        )?;
        drop(publish_guard);
        Ok(cs)
    }

    /// Shared tail of the single-module and package publish paths: record the
    /// published modules, parse VM effects, charge publish gas, and optionally
    /// persist the runtime state.
    #[allow(clippy::too_many_arguments)]
    fn finish_publish_session<'a>(
        &self,
        mut cs: ChangeSet,
        move_changeset: move_core_types::effects::ChangeSet,
        events: &[move_core_types::effects::Event],
        vm_gas_used: u64,
        sender: AccountAddress,
        module_ids: impl IntoIterator<Item = &'a ModuleId>,
        module_size: usize,
        gas_info: Option<(u64, u64)>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        for module_id in module_ids {
            cs.publish_module(sender, module_id.name().to_string());
        }
        self.parse_move_changeset(&move_changeset, &mut cs);
        self.parse_move_events(events, &mut cs);

        if let Some((gas_limit, gas_price)) = gas_info {
            let gas_op = GasOperation::PublishModule { module_size };
            let (written, deleted) = self.calculate_storage_impact(&move_changeset, &cs, None)?;
            self.apply_gas_info(&mut cs, gas_limit, gas_price, gas_op, written, deleted)?;
        }
        cs.set_gas_used(cs.gas_used.max(vm_gas_used));

        if persist_runtime_state {
            self.apply_move_changeset(move_changeset)?;
            self.persist_created_objects(&cs)?;
            self.persist_deleted_objects(&cs)?;
            self.reload_vm_cache()?;
        }

        Ok(cs)
    }

    /// Whether a compiled module declares an `init` function.
    fn module_declares_init(compiled: &CompiledModule) -> bool {
        compiled.function_defs().iter().any(|func_def| {
            let handle = compiled.function_handle_at(func_def.function);
            compiled.identifier_at(handle.name).as_str() == "init"
        })
    }

    fn package_init_tx_hash(base_tx_hash: Option<&[u8]>, module_id: &ModuleId) -> Vec<u8> {
        let mut input = b"kanari-package-init-v1".to_vec();
        if let Some(base_tx_hash) = base_tx_hash {
            input.extend_from_slice(base_tx_hash);
        }
        input.extend_from_slice(module_id.address().as_ref());
        input.extend_from_slice(module_id.name().as_bytes());
        hash_data_blake3(&input).to_vec()
    }

    /// Execute a freshly published module's `init` inside an already-open
    /// publish session and merge its effects into the publish changeset.
    ///
    /// Running in-session (instead of a follow-up session) is what makes
    /// this work in every mode: the module is visible because this very
    /// session published it, so dry-run previews include init effects and
    /// persisted commits behave identically. Gas comes from the caller's
    /// shared meter, so init compute is paid for, never free. Any init
    /// failure aborts the whole publish (atomic).
    ///
    /// Call only for the [`Publish`](ModulePublishIntent::Publish) intent:
    /// upgrades and bootstraps must never re-run `init`, which is what makes
    /// one-time initialization (treasury creation, registry seeding)
    /// trustworthy. `publisher` becomes `tx_context::sender` inside `init`.
    #[allow(clippy::too_many_arguments)]
    fn execute_init_in_publish_session(
        &self,
        session: &mut Session<'_, '_, KanariMoveResolver>,
        module_id: &ModuleId,
        publisher: AccountAddress,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        metered_gas: &mut crate::kanari_gas_meter::KanariGasMeter,
        cs: &mut ChangeSet,
    ) -> Result<()> {
        use move_core_types::language_storage::TypeTag;

        let ident = IdentStr::new("init").require("Invalid init function name")?;
        let func = session
            .load_function(module_id, ident, &[])
            .require("Failed to load init function")?;

        // Build args: one empty slot per non-context parameter (the entry
        // machinery synthesizes one-time-witness bytes into empty slots),
        // then `TxContext` itself. Anything else is rejected rather than
        // mis-executed. References are unwrapped first: `TxContext` always
        // arrives as `&mut`, mirroring the entry-function binding rules.
        let mut final_args = Vec::new();
        for param_type in func.parameters.iter() {
            let base_type = match param_type {
                RuntimeType::Reference(inner) | RuntimeType::MutableReference(inner) => {
                    inner.as_ref()
                }
                base => base,
            };
            let tag = session.get_type_tag(base_type).ok();
            let is_tx_context =
                matches!(tag, Some(TypeTag::Struct(ref st)) if Self::is_tx_context_struct(st));
            if is_tx_context {
                continue;
            }
            match tag {
                Some(TypeTag::Struct(_)) => {
                    let layout = session
                        .type_to_type_layout(base_type)
                        .require("Failed to resolve init parameter layout")?;
                    match Self::synthesize_otw_bytes_from_layout(&layout) {
                        Some(otw) => final_args.push(otw),
                        None => anyhow::bail!(
                            "Unsupported init parameter: only one-time-witness and TxContext parameters are allowed"
                        ),
                    }
                }
                _ => anyhow::bail!(
                    "Unsupported init parameter: only one-time-witness and TxContext parameters are allowed"
                ),
            }
        }
        let tx_context_bytes =
            self.build_tx_context_bytes(Some(publisher), timestamp, tx_hash.as_deref())?;
        final_args.push(tx_context_bytes);

        session
            .execute_function_bypass_visibility(module_id, ident, vec![], final_args, metered_gas)
            .require("Module init execution failed")?;

        let SessionNativeOutputs {
            transferred,
            events,
            saved,
            deleted,
            dynamic_fields,
            borrowed,
        } = SessionNativeOutputs::drain(session.get_native_extensions());

        self.add_transferred_objects_to_changeset(cs, transferred, false, None)?;
        SessionNativeOutputs::apply_simple_effects(cs, events, deleted, dynamic_fields);

        let empty_mutables: Vec<LoadedMutableObject> = Vec::new();
        let mut processed_ids = HashSet::new();
        self.upsert_saved_and_borrowed_objects(
            cs,
            &empty_mutables,
            saved,
            borrowed,
            "init-saved",
            "init-borrowed",
            &mut processed_ids,
        )?;
        Ok(())
    }

    fn verify_module_publish_safety(
        &self,
        sender: AccountAddress,
        module_id: &ModuleId,
        compiled: &CompiledModule,
        module_bytes: &[u8],
        intent: ModulePublishIntent,
    ) -> Result<()> {
        if module_id.address() != &sender {
            anyhow::bail!(
                "Module publish rejected: sender {} cannot publish module {}. \
                 Module address must match the transaction sender.",
                sender,
                module_id
            );
        }

        let old_bytes = self.verify_module_publish_intent(module_id, intent)?;
        verify_module_unmetered(compiled).require("Module bytecode verification failed")?;
        self.verify_module(compiled)?;
        if let Some(old_bytes) = old_bytes {
            self.verify_module_upgrade_compatibility(
                module_id,
                compiled,
                module_bytes,
                &old_bytes,
            )?;
        }
        Ok(())
    }

    fn verify_module_publish_intent(
        &self,
        module_id: &ModuleId,
        intent: ModulePublishIntent,
    ) -> Result<Option<Vec<u8>>> {
        let old_bytes = self.state.try_get_module(module_id)?;
        match (intent, old_bytes) {
            (ModulePublishIntent::Bootstrap, old_bytes) => Ok(old_bytes),
            (ModulePublishIntent::Publish, Some(_)) => {
                anyhow::bail!(
                    "Module publish rejected: module {} already exists. Use upgrade for compatibility-checked replacement.",
                    module_id
                );
            }
            (ModulePublishIntent::Publish, None) => Ok(None),
            (ModulePublishIntent::Upgrade, None) => {
                anyhow::bail!(
                    "Module upgrade rejected: module {} does not exist. Publish the package before upgrading it.",
                    module_id
                );
            }
            (ModulePublishIntent::Upgrade, Some(old_bytes)) => Ok(Some(old_bytes)),
        }
    }

    fn verify_module_upgrade_compatibility(
        &self,
        module_id: &ModuleId,
        compiled: &CompiledModule,
        module_bytes: &[u8],
        old_bytes: &[u8],
    ) -> Result<()> {
        if old_bytes == module_bytes {
            return Ok(());
        }

        let old_compiled = CompiledModule::deserialize_with_defaults(old_bytes)?;
        let old_norm = normalized::Module::new(&old_compiled);
        let new_norm = normalized::Module::new(compiled);

        Compatibility::full_check()
            .check(&old_norm, &new_norm)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Incompatible module upgrade for {} rejected: {:?}. \
                     Existing resources/objects must remain layout-compatible; publish a compatible module or run an explicit migration path first.",
                    module_id,
                    e
                )
            })
    }
}
