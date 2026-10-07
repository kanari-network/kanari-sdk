// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MoveRuntime {
    /// Execute a Move entry function with the given arguments and context.
    pub fn execute_entry_function(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            ExecutionOptions::new(sender, gas_info, timestamp, None),
        )
        .map(|(changeset, _)| changeset)
    }

    /// Execute a Move entry function with full object context and persistence control.
    pub fn execute_entry_function_with_object_context_and_persistence(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        context: EntryFunctionObjectContext,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_with_object_context_returns(
            module_id,
            function_name,
            type_args,
            args,
            context,
        )
        .map(|(changeset, _)| changeset)
    }

    /// Same as
    /// [`execute_entry_function_with_object_context_and_persistence`](Self::execute_entry_function_with_object_context_and_persistence),
    /// but additionally returns the entry function's pure return values
    /// (BCS bytes per value) for callers that compose calls, such as the
    /// programmable transaction interpreter threading a `u64` across commands.
    pub fn execute_entry_function_with_object_context_returns(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        context: EntryFunctionObjectContext,
    ) -> Result<(ChangeSet, Vec<Vec<u8>>)> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            ExecutionOptions::new(
                context.sender,
                context.gas_info,
                context.timestamp,
                context.tx_hash,
            )
            .with_object_inputs(context.object_inputs)
            .with_persistence(context.persist_runtime_state)
            .with_state_overlay(context.state_overlay),
        )
    }

    pub(super) fn execute_system_function_with_tx_hash_and_persistence(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        sender: Option<AccountAddress>,
        gas_info: Option<(u64, u64)>,
        timestamp: Option<u64>,
        tx_hash: Option<Vec<u8>>,
        persist_runtime_state: bool,
    ) -> Result<ChangeSet> {
        self.execute_entry_function_internal(
            module_id,
            function_name,
            type_args,
            args,
            ExecutionOptions::new(sender, gas_info, timestamp, tx_hash)
                .with_persistence(persist_runtime_state)
                .bypass_entry_check(),
        )
        .map(|(changeset, _)| changeset)
    }

    pub(super) fn execute_entry_function_internal(
        &self,
        module_id: &ModuleId,
        function_name: &str,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        options: ExecutionOptions,
    ) -> Result<(ChangeSet, Vec<Vec<u8>>)> {
        let ExecutionOptions {
            sender,
            gas_info,
            timestamp,
            tx_hash,
            object_inputs,
            persist_runtime_state,
            bypass_entry_check,
            state_overlay,
        } = options;

        let vm_guard = self.read_vm();
        let (resolver, resolver_trace) = self
            .resolver
            .tracing_clone_with_overlay(state_overlay.clone());
        let mut session =
            self.create_session_with_resolver(&vm_guard, resolver, state_overlay.clone());
        let mut deterministic_reads =
            std::collections::BTreeSet::from([crate::common::keys::module_key(
                module_id.address(),
                module_id.name().as_str(),
            )
            .into_bytes()]);

        // Preload object arguments so native object borrows can resolve them from extensions.
        self.preload_objects_for_execution(
            &mut session,
            &object_inputs,
            sender,
            state_overlay.as_deref(),
        )?;

        let explicit_object_ids: HashSet<String> = object_inputs
            .iter()
            .map(|input| input.object_ref.object_id.clone())
            .collect();
        let mut explicit_object_bindings = object_inputs.iter();

        let mut ty_args_loaded = vec![];
        for tag in type_args.iter() {
            ty_args_loaded.push(
                session
                    .load_type(tag)
                    .require("Object storage operation failed")?,
            );
        }

        let ident = IdentStr::new(function_name).require("Invalid function name")?;

        let mut final_args = Self::preprocess_entry_args(args);
        deterministic_reads.extend(
            final_args
                .iter()
                .filter(|arg| arg.len() == AccountAddress::LENGTH)
                .map(|arg| format!("object:0x{}", hex::encode(arg)).into_bytes()),
        );
        self.preload_object_ids_from_args(
            &mut session,
            &final_args,
            sender,
            state_overlay.as_deref(),
        )?;
        let tx_context_bytes =
            self.build_tx_context_bytes(sender, timestamp, tx_hash.as_deref())?;

        let mut loaded_mutable_objects: Vec<LoadedMutableObject> = Vec::new();

        if let Ok(func) = session.load_function(module_id, ident, &ty_args_loaded) {
            let probe = ParamProbe::new(&session);
            let binding_requirements = probe.binding_requirements(&func.parameters);

            // System functions (bypass_entry_check) bind object references via the
            // plain 32-byte id argument path below rather than via declared
            // object_inputs, so the declared-binding count check does not apply.
            if !bypass_entry_check {
                Self::validate_object_input_bindings(&object_inputs, &binding_requirements, true)?;
            }

            for (i, param_type) in func.parameters.iter().enumerate() {
                let object_mutability = MoveRuntime::object_param_mutability(param_type, |t| {
                    probe.is_key_struct_param(t)
                });

                if i >= final_args.len() {
                    let needs_declared_object_input = !bypass_entry_check
                        && object_mutability.is_some()
                        && !probe.is_tx_context_param(param_type);
                    if needs_declared_object_input {
                        final_args.resize_with(i + 1, Vec::new);
                    } else {
                        break;
                    }
                }

                let mut bound_from_explicit_input = false;

                if final_args[i].is_empty()
                    && let Some(TypeTag::Struct(struct_tag)) = probe.type_tag_for_param(param_type)
                    && struct_tag.address == *module_id.address()
                    && struct_tag.module.as_str() == module_id.name().as_str()
                    && struct_tag.name.as_str() == module_id.name().as_str().to_ascii_uppercase()
                    && let Ok(layout) = session.type_to_type_layout(param_type)
                    && let Some(otw_bytes) = Self::synthesize_otw_bytes_from_layout(&layout)
                {
                    final_args[i] = otw_bytes;
                }

                if object_mutability.is_some()
                    && let Some(TypeTag::Struct(_)) = probe.type_tag_for_param(param_type)
                    && let Some(explicit_input) = explicit_object_bindings.next()
                {
                    let explicit_addr =
                        AccountAddress::from_hex_literal(&explicit_input.object_ref.object_id)
                            .with_context(|| {
                                format!(
                                    "Invalid explicit object input {} for parameter {}",
                                    explicit_input.object_ref.object_id, i
                                )
                            })?;
                    final_args[i] = explicit_addr.to_vec();
                    bound_from_explicit_input = true;
                }

                let is_potential_id = final_args[i].len() == 32;

                if is_potential_id
                    && object_mutability.is_some()
                    && !probe.is_tx_context_param(param_type)
                {
                    let Ok(object_addr) = AccountAddress::from_bytes(final_args[i].as_slice())
                    else {
                        continue;
                    };
                    let object_id = object_addr.to_hex_literal();
                    if !explicit_object_ids.is_empty() && !explicit_object_ids.contains(&object_id)
                    {
                        continue;
                    }

                    if let Some(stored_obj) =
                        self.get_object_for_execution(&object_id, state_overlay.as_deref())?
                    {
                        // Raw address args (not declared in object_inputs) may
                        // only touch sender-owned / system / zero-address
                        // objects. Objects declared explicitly in object_inputs
                        // opted into dependency tracking, so cross-owner reads
                        // (e.g. a co-owner approving a multisig proposal) are
                        // allowed here — Move-level authorization (is_owner,
                        // capability checks) still applies inside the called
                        // function.
                        let declared_explicitly =
                            bound_from_explicit_input || explicit_object_ids.contains(&object_id);
                        if !declared_explicitly && let Some(s_addr) = sender {
                            let sys_addr = KanariAddress::kanari_system_account_address();
                            let std_addr = KanariAddress::std_account_address();
                            if stored_obj.owner != s_addr
                                && stored_obj.owner != AccountAddress::ZERO
                                && stored_obj.owner != sys_addr
                                && stored_obj.owner != std_addr
                            {
                                return Err(anyhow::anyhow!("Ownership verification failed"));
                            }
                        }

                        final_args[i] = stored_obj.data.clone();

                        if matches!(
                            param_type,
                            RuntimeType::MutableReference(_)
                                | RuntimeType::Struct(_)
                                | RuntimeType::StructInstantiation(_)
                        ) {
                            loaded_mutable_objects.push((
                                i,
                                stored_obj.id.clone(),
                                stored_obj.owner,
                                stored_obj.owner_kind.clone(),
                                stored_obj.type_name.clone(),
                                stored_obj.version,
                            ));
                        }
                    }
                }
            }
            if func.parameters.len() == final_args.len() + 1
                && let Some(last_param_type) = func.parameters.last()
                && let Some(TypeTag::Struct(struct_tag)) = probe.type_tag_for_param(last_param_type)
                && Self::is_tx_context_struct(&struct_tag)
            {
                final_args.push(tx_context_bytes);
            }
        }

        // Extensions are already added by create_session_with_storage_ext() - no need to add again
        let provided_gas_limit = gas_info.map(|(limit, _)| limit).unwrap_or(1_000_000);
        let mut metered_gas = crate::kanari_gas_meter::KanariGasMeter::new(provided_gas_limit);
        let execution_result = if bypass_entry_check {
            session.execute_function_bypass_visibility(
                module_id,
                ident,
                ty_args_loaded,
                final_args,
                &mut metered_gas,
            )
        } else {
            session.execute_entry_function(
                module_id,
                ident,
                ty_args_loaded,
                final_args,
                &mut metered_gas,
            )
        };
        let vm_gas_used = metered_gas.gas_used();

        let mut cs = ChangeSet::new();

        match execution_result {
            Ok(return_values) => {
                // Pure return values (as opposed to mutable-reference
                // writebacks below) are surfaced to callers that compose
                // calls, e.g. programmable transactions threading a `u64`
                // across commands.
                let pure_return_values: Vec<Vec<u8>> = return_values
                    .return_values
                    .iter()
                    .map(|(bytes, _)| bytes.clone())
                    .collect();
                // Extract data from native extensions before finishing the session
                let SessionNativeOutputs {
                    transferred,
                    events: native_events,
                    saved,
                    deleted: native_deleted,
                    dynamic_fields: native_dynamic_fields,
                    borrowed,
                } = SessionNativeOutputs::drain(session.get_native_extensions());

                let (res, _) = session.finish();
                let (move_changeset, events) = res.require("exec error")?;

                for (addr, account_changes) in move_changeset.accounts() {
                    for op in account_changes.resources().values() {
                        if let move_core_types::effects::Op::Delete = op {
                            cs.add_deleted_object(addr.to_hex_literal());
                        }
                    }
                }

                self.parse_move_changeset(&move_changeset, &mut cs);
                self.parse_move_events(&events, &mut cs);

                let mut processed_ids = HashSet::new();

                for (idx, data, _) in return_values.mutable_reference_outputs {
                    if let Some((_, id, owner, owner_kind, type_name, version)) =
                        loaded_mutable_objects
                            .iter()
                            .find(|(i, _, _, _, _, _)| *i == idx as usize)
                    {
                        self.upsert_created_object(
                            &mut cs,
                            *owner,
                            owner_kind.clone(),
                            id,
                            type_name,
                            data.clone(),
                            version + 1,
                            "writeback",
                        );
                        processed_ids.insert(id.clone());
                    }
                }

                self.upsert_saved_and_borrowed_objects(
                    &mut cs,
                    &loaded_mutable_objects,
                    saved,
                    borrowed,
                    "saved",
                    "borrowed_mut",
                    &mut processed_ids,
                )?;

                self.add_transferred_objects_to_changeset(
                    &mut cs,
                    transferred,
                    persist_runtime_state,
                    state_overlay.as_ref(),
                )?;

                SessionNativeOutputs::apply_simple_effects(
                    &mut cs,
                    native_events,
                    native_deleted,
                    native_dynamic_fields,
                );

                if let Some((gas_limit, gas_price)) = gas_info {
                    let complexity = 1;
                    let gas_op = GasOperation::ExecuteFunction { complexity };
                    let (written, deleted) = self.calculate_storage_impact(
                        &move_changeset,
                        &cs,
                        state_overlay.as_ref(),
                    )?;
                    self.apply_gas_info(&mut cs, gas_limit, gas_price, gas_op, written, deleted)?;
                }
                cs.set_gas_used(cs.gas_used.max(vm_gas_used));

                if persist_runtime_state {
                    self.apply_move_changeset(move_changeset)?;
                    self.persist_created_objects(&cs)?;
                    self.persist_deleted_objects(&cs)?;
                }

                let reads = match resolver_trace.lock() {
                    Ok(reads) => reads.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                };
                cs.record_resolver_reads(reads);
                cs.record_resolver_reads(deterministic_reads);
                Ok((cs, pure_return_values))
            }
            Err(e) => {
                if let Some((gas_limit, gas_price)) = gas_info {
                    let penalty_complexity = 5;
                    let _ = self.apply_gas_info(
                        &mut cs,
                        gas_limit,
                        gas_price,
                        GasOperation::ExecuteFunction {
                            complexity: penalty_complexity,
                        },
                        0,
                        0,
                    );
                    if persist_runtime_state {
                        self.persist_created_objects(&cs)?;
                    }
                }
                Err(anyhow::anyhow!("exec error: {:?}", e))
            }
        }
    }

    /// Create a session with the native extensions required by the runtime.
    pub(super) fn create_session_with_storage_ext<'r>(
        &'r self,
        vm_guard: &'r std::sync::RwLockReadGuard<'r, MoveVM>,
    ) -> Session<'r, 'r, KanariMoveResolver> {
        self.create_session_with_resolver(vm_guard, self.resolver.clone(), None)
    }

    fn create_session_with_resolver<'r>(
        &'r self,
        vm_guard: &'r std::sync::RwLockReadGuard<'r, MoveVM>,
        resolver: KanariMoveResolver,
        state_overlay: Option<crate::StateOverlay>,
    ) -> Session<'r, 'r, KanariMoveResolver> {
        // Register the core extensions used by object, event, and dynamic-field natives.
        let mut extensions = NativeContextExtensions::default();
        extensions.add(DynamicFieldsExt::default());
        extensions.add(self.dynamic_field_storage_ext(state_overlay));
        extensions.add(EventsExt::default());
        extensions.add(SavedObjectsExt::default());
        extensions.add(DeletedObjectsExt::default());
        extensions.add(TransferredObjectsExt::default());

        // Track objects loaded and mutated through object native functions.
        extensions.add(LoadedObjectsExt::default());
        extensions.add(BorrowedObjectsExt::default());

        vm_guard.new_session_with_extensions(resolver, extensions)
    }
}
