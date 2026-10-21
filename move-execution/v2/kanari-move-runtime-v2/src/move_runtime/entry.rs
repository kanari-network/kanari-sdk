// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

use kanari_types::coin::CoinModule;

/// Gas budget for a deny-policy read (`deny_list_contains` /
/// `deny_list_paused`) inside either runtime gate.
///
/// Named and pinned because these reads are consensus-critical: if the budget
/// regresses below what the framework read costs at realistic deny-set sizes,
/// fail-closed semantics would reject *every* movement of a regulated coin.
/// `tests/deny_list_batch_capacity.rs` pins the current cost curve by
/// executing reads against a populated deny list and proving the budget
/// comfortably covers it; raise it deliberately if the framework read grows.
pub(crate) const DENY_POLICY_GAS_BUDGET: u64 = 1_000_000;

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
            .map(|input| {
                crate::common::ids::canonical_object_id_or_raw(&input.object_ref.object_id)
            })
            .collect();
        let mut used_explicit_object_bindings = HashSet::new();

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
        // Well-known singletons (deny list) resolve without being declared.
        self.preload_well_known_singletons(&mut session, state_overlay.as_deref())?;
        let tx_context_bytes =
            self.build_tx_context_bytes(sender, timestamp, tx_hash.as_deref())?;

        let mut loaded_mutable_objects: Vec<LoadedMutableObject> = Vec::new();

        if let Ok(func) = session.load_function(module_id, ident, &ty_args_loaded) {
            // Protocol-level regulated-coin input policy: denied senders and
            // paused coin types cannot use Coin objects as transaction inputs,
            // no matter which entry function is called. Evaluated here (from
            // the raw inputs, before binding) so entry code cannot bypass it.
            // Placed before `probe` borrows the session.
            self.enforce_coin_input_policy(
                &mut session,
                sender,
                &object_inputs,
                &final_args,
                state_overlay.as_deref(),
                bypass_entry_check,
            )?;

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

                let explicit_binding_index = if matches!(
                    probe.type_tag_for_param(param_type),
                    Some(TypeTag::Struct(_))
                ) {
                    let argument_address = AccountAddress::from_bytes(&final_args[i]).ok();
                    let matching_input = argument_address.and_then(|address| {
                        object_inputs.iter().enumerate().find_map(|(index, input)| {
                            if used_explicit_object_bindings.contains(&index) {
                                return None;
                            }
                            AccountAddress::from_hex_literal(&input.object_ref.object_id)
                                .ok()
                                .filter(|input_address| *input_address == address)
                                .map(|_| index)
                        })
                    });
                    matching_input.or_else(|| {
                        object_mutability.and_then(|_| {
                            object_inputs.iter().enumerate().find_map(|(index, _)| {
                                (!used_explicit_object_bindings.contains(&index)).then_some(index)
                            })
                        })
                    })
                } else {
                    None
                };
                let bound_object_mutability = object_mutability
                    .or_else(|| explicit_binding_index.map(|index| object_inputs[index].mutable));

                if let Some(binding_index) = explicit_binding_index {
                    used_explicit_object_bindings.insert(binding_index);
                    let explicit_input = &object_inputs[binding_index];
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
                    && bound_object_mutability.is_some()
                    && !probe.is_tx_context_param(param_type)
                {
                    let Ok(object_addr) = AccountAddress::from_bytes(final_args[i].as_slice())
                    else {
                        continue;
                    };
                    let object_id = object_addr.to_hex_literal();
                    if !bound_from_explicit_input
                        && !explicit_object_ids.is_empty()
                        && !explicit_object_ids
                            .contains(&crate::common::ids::canonical_object_id_or_raw(&object_id))
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
                        let declared_explicitly = bound_from_explicit_input
                            || explicit_object_ids.contains(
                                &crate::common::ids::canonical_object_id_or_raw(&object_id),
                            );
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
                    } else {
                        anyhow::bail!(
                            "Object input {object_id} for entry parameter {i} of {module_id}::{function_name} was not found"
                        );
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

            if final_args.len() == func.parameters.len() {
                for (index, (param_type, arg)) in
                    func.parameters.iter().zip(&final_args).enumerate()
                {
                    let value_type = match param_type {
                        RuntimeType::Reference(inner) | RuntimeType::MutableReference(inner) => {
                            inner
                        }
                        _ => param_type,
                    };
                    let Ok(layout) = session.type_to_type_layout(value_type) else {
                        continue;
                    };
                    if move_vm_types::values::Value::simple_deserialize(arg, &layout).is_none() {
                        let expected_type = probe
                            .type_tag_for_param(param_type)
                            .map(|tag| tag.to_string())
                            .unwrap_or_else(|| format!("{param_type:?}"));
                        anyhow::bail!(
                            "Failed to deserialize Move entry argument {index} for {module_id}::{function_name}: expected {expected_type}, received {} bytes",
                            arg.len()
                        );
                    }
                }
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
                    claimed_witnesses,
                    child_links,
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

                if !bypass_entry_check {
                    self.enforce_coin_output_policy(
                        &vm_guard,
                        &cs,
                        &transferred,
                        state_overlay.clone(),
                    )?;
                }

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

                self.record_session_metadata_writes(
                    &mut cs,
                    claimed_witnesses,
                    child_links,
                    state_overlay.as_deref(),
                )?;

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
                    self.apply_and_persist_move_changeset(move_changeset, &cs)?;
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
        extensions.add(self.dynamic_field_storage_ext(state_overlay.clone()));
        extensions.add(EventsExt::default());
        extensions.add(SavedObjectsExt::default());
        extensions.add(DeletedObjectsExt::default());
        extensions.add(TransferredObjectsExt::default());

        // Track objects loaded and mutated through object native functions.
        extensions.add(LoadedObjectsExt::default());
        extensions.add(BorrowedObjectsExt::default());

        // Type-convention and dynamic-object-field metadata registries.
        extensions.add(self.claim_witness_ext(state_overlay.clone()));
        extensions.add(self.child_index_ext(state_overlay));

        vm_guard.new_session_with_extensions(resolver, extensions)
    }

    /// Execute a framework `deny_list_*` read and decode its `bool` result.
    ///
    /// A read that cannot be performed (unloadable or failing function) is
    /// logged and yields `Ok(None)`; the caller decides how to fail open. Real
    /// state errors surface from the main execution path itself.
    fn read_deny_list_flag(
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        coin_module: &ModuleId,
        function_name: &str,
        coin_type: RuntimeType,
        args: Vec<Vec<u8>>,
        gas: &mut crate::kanari_gas_meter::KanariGasMeter,
        coin_type_name: &str,
    ) -> Result<Option<bool>> {
        let ident = IdentStr::new(function_name)?;
        match session.execute_function_bypass_visibility(
            coin_module,
            ident,
            vec![coin_type],
            args,
            gas,
        ) {
            Ok(values) => Ok(Some(
                values
                    .return_values
                    .first()
                    .map(|(bytes, _)| !bytes.is_empty() && bytes[0] != 0)
                    .unwrap_or(false),
            )),
            Err(error) => {
                log::warn!(
                    "[RUNTIME] deny policy read {function_name} failed for {coin_type_name}: {error:?}"
                );
                Ok(None)
            }
        }
    }

    /// Protocol-level regulated-coin input policy.
    ///
    /// A denied sender — or a globally paused coin type — may not use
    /// `Coin<T>` objects as transaction inputs, regardless of which entry
    /// function is called. Unchecked transfer paths (`coin::split` +
    /// `public_transfer`, plain `pay::split_and_transfer`) skip the
    /// Move-level checks by design, so this host-side gate is what makes
    /// deny lists enforceable at consensus rather than advisory.
    ///
    /// Evaluated by executing the framework's own `deny_list_contains` /
    /// `deny_list_paused` reads in this session, so deny-key rendering
    /// always matches the framework (no duplicated key logic). Skipped for
    /// system/init executions and when no sender is set. Unresolvable
    /// inputs fail **closed** with an error: deny state that cannot be
    /// verified must not let a transaction through. The main execution
    /// surfaces real state errors itself.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn enforce_coin_input_policy(
        &self,
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        sender: Option<AccountAddress>,
        object_inputs: &[ObjectInput],
        raw_args: &[Vec<u8>],
        overlay: Option<&std::collections::BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
        bypass_entry_check: bool,
    ) -> Result<()> {
        use std::str::FromStr;

        if bypass_entry_check {
            return Ok(());
        }
        let Some(sender) = sender else {
            return Ok(());
        };

        // Candidate Coin inputs: declared object inputs plus raw 32-byte
        // args resolving to stored Coin objects (undeclared reads bind
        // through the same path, so they must be policed too).
        let mut coin_types: Vec<String> = Vec::new();
        for input in object_inputs {
            if let Some(stored) =
                self.get_object_for_execution(&input.object_ref.object_id, overlay)?
                && let Some(inner) = CoinModule::inner_type(&stored.type_name)
            {
                coin_types.push(inner);
            }
        }
        for arg in raw_args {
            if arg.len() != AccountAddress::LENGTH {
                continue;
            }
            let object_id = format!("0x{}", hex::encode(arg));
            if let Some(stored) = self.get_object_for_execution(&object_id, overlay)?
                && let Some(inner) = CoinModule::inner_type(&stored.type_name)
            {
                coin_types.push(inner);
            }
        }
        coin_types.sort();
        coin_types.dedup();
        if coin_types.is_empty() {
            return Ok(());
        }

        let Some(singleton) =
            self.get_object_for_execution(&Self::deny_list_singleton_full_id(), overlay)?
        else {
            // Fail closed: deny state must be verifiable for any coin
            // movement. A missing singleton after genesis is corruption.
            anyhow::bail!(
                "deny-list policy: shared deny-list singleton {} is missing; \
                 refusing to execute a transaction that moves regulated coins",
                Self::deny_list_singleton_full_id(),
            );
        };
        let coin_module = ModuleId::new(
            kanari_types::address::Address::kanari_system_account_address(),
            move_core_types::identifier::Identifier::new(CoinModule::COIN_MODULE)?,
        );
        let coin_fns = CoinModule::function_names();
        let sender_bytes = bcs::to_bytes(&sender)?;
        let mut policy_gas = crate::kanari_gas_meter::KanariGasMeter::new(DENY_POLICY_GAS_BUDGET);

        for inner in &coin_types {
            let tag = match move_core_types::language_storage::TypeTag::from_str(inner) {
                Ok(tag) => tag,
                Err(error) => {
                    anyhow::bail!(
                        "deny-list policy: unparsable coin type {inner} on transaction \
                         input ({error}); refusing to verify deny state",
                    );
                }
            };
            let ty = match session.load_type(&tag) {
                Ok(ty) => ty,
                Err(error) => {
                    anyhow::bail!(
                        "deny-list policy: coin type {inner} on transaction input \
                         cannot be loaded ({error:?}); refusing to verify deny state",
                    );
                }
            };

            // Each read fails closed: a read that cannot run rejects the
            // transaction instead of waving it through.
            let paused = Self::read_deny_list_flag(
                session,
                &coin_module,
                coin_fns.deny_list_paused,
                ty.clone(),
                vec![singleton.data.clone()],
                &mut policy_gas,
                inner,
            )?;
            let Some(paused) = paused else {
                anyhow::bail!(
                    "deny-list policy: the paused state for coin type {inner} \
                     could not be read; refusing the transaction (fail-closed)",
                );
            };
            if paused {
                anyhow::bail!(
                    "transaction sender {} is blocked: regulated coin type {} is globally paused (deny-list policy)",
                    sender.to_hex_literal(),
                    inner,
                );
            }

            let denied = Self::read_deny_list_flag(
                session,
                &coin_module,
                coin_fns.deny_list_contains,
                ty,
                vec![singleton.data.clone(), sender_bytes.clone()],
                &mut policy_gas,
                inner,
            )?;
            let Some(denied) = denied else {
                anyhow::bail!(
                    "deny-list policy: the denied state for coin type {inner} \
                     could not be read; refusing the transaction (fail-closed)",
                );
            };
            if denied {
                anyhow::bail!(
                    "transaction sender {} is denied for regulated coin {} (deny-list policy)",
                    sender.to_hex_literal(),
                    inner,
                );
            }
        }
        Ok(())
    }

    /// Protocol-level regulated-coin output policy.
    ///
    /// Companion to `enforce_coin_input_policy`: a denied recipient — or a
    /// globally paused coin type — may not end up owning a `Coin<T>` object,
    /// regardless of which entry path delivered it (plain `coin::transfer`,
    /// `pay::split_and_transfer`, split + transfer, ...). The checked Move
    /// paths reject at the call site; this host-side gate closes the
    /// unchecked ones.
    ///
    /// Runs before the first store write in the success path, scanning the
    /// already-assembled changeset (writebacks, saved/borrowed objects) plus
    /// the drained `transferred` list, whose members are persisted directly
    /// by `add_transferred_objects_to_changeset` after this check. Every
    /// candidate whose type parses as `Coin<T>` and whose final owner is an
    /// address is checked by executing the framework's own
    /// `deny_list_contains` / `deny_list_paused` reads in a fresh session,
    /// so deny-key rendering always matches the framework. Unresolvable
    /// reads fail closed with an error, mirroring the input policy.
    fn enforce_coin_output_policy(
        &self,
        vm_guard: &std::sync::RwLockReadGuard<'_, MoveVM>,
        cs: &ChangeSet,
        transferred: &[TransferredObject],
        state_overlay: Option<crate::StateOverlay>,
    ) -> Result<()> {
        use std::str::FromStr;

        let mut targets: Vec<(String, AccountAddress)> = cs
            .created_objects
            .iter()
            .filter_map(|(_, created)| {
                Self::coin_recipient_candidate(&created.type_, created.owner)
            })
            .collect();
        for obj in transferred
            .iter()
            .filter(|obj| obj.should_persist && !obj.is_frozen && !obj.is_shared)
        {
            if let Some(pair) = Self::coin_recipient_candidate(&obj.object_type, obj.recipient) {
                targets.push(pair);
            }
        }
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            return Ok(());
        }

        let Some(singleton) = self.get_object_for_execution(
            &Self::deny_list_singleton_full_id(),
            state_overlay.as_deref(),
        )?
        else {
            anyhow::bail!(
                "deny-list output policy: shared deny-list singleton {} is missing; \
                 refusing the transaction that moves regulated coins",
                Self::deny_list_singleton_full_id(),
            );
        };
        let (resolver, _resolver_trace) = self
            .resolver
            .tracing_clone_with_overlay(state_overlay.clone());
        let mut session = self.create_session_with_resolver(vm_guard, resolver, state_overlay);
        let coin_module = ModuleId::new(
            kanari_types::address::Address::kanari_system_account_address(),
            move_core_types::identifier::Identifier::new(CoinModule::COIN_MODULE)?,
        );
        let coin_fns = CoinModule::function_names();
        let mut policy_gas = crate::kanari_gas_meter::KanariGasMeter::new(DENY_POLICY_GAS_BUDGET);

        for (inner, recipient) in targets {
            let tag = match move_core_types::language_storage::TypeTag::from_str(&inner) {
                Ok(tag) => tag,
                Err(error) => {
                    anyhow::bail!(
                        "deny-list output policy: unparsable coin type {inner} \
                         created for {} ({error}); refusing the transaction",
                        recipient.to_hex_literal(),
                    );
                }
            };
            let ty = match session.load_type(&tag) {
                Ok(ty) => ty,
                Err(error) => {
                    anyhow::bail!(
                        "deny-list output policy: coin type {inner} created for {} \
                         cannot be loaded ({error:?}); refusing the transaction",
                        recipient.to_hex_literal(),
                    );
                }
            };

            let paused = Self::read_deny_list_flag(
                &mut session,
                &coin_module,
                coin_fns.deny_list_paused,
                ty.clone(),
                vec![singleton.data.clone()],
                &mut policy_gas,
                &inner,
            )?;
            let Some(paused) = paused else {
                anyhow::bail!(
                    "deny-list output policy: the paused state for coin type {inner} \
                     could not be read; refusing the transaction (fail-closed)",
                );
            };
            if paused {
                anyhow::bail!(
                    "recipient {} cannot receive coin type {}: it is globally paused (deny-list policy)",
                    recipient.to_hex_literal(),
                    inner,
                );
            }

            let recipient_bytes = bcs::to_bytes(&recipient)?;
            let denied = Self::read_deny_list_flag(
                &mut session,
                &coin_module,
                coin_fns.deny_list_contains,
                ty,
                vec![singleton.data.clone(), recipient_bytes],
                &mut policy_gas,
                &inner,
            )?;
            let Some(denied) = denied else {
                anyhow::bail!(
                    "deny-list output policy: the denied state for coin type {inner} \
                     could not be read; refusing the transaction (fail-closed)",
                );
            };
            if denied {
                anyhow::bail!(
                    "recipient {} is denied for regulated coin {} (deny-list policy)",
                    recipient.to_hex_literal(),
                    inner,
                );
            }
        }
        Ok(())
    }

    /// Output-policy candidate: a coin-type object landing at a non-zero
    /// address owner.
    fn coin_recipient_candidate(
        object_type: &str,
        owner: AccountAddress,
    ) -> Option<(String, AccountAddress)> {
        if owner == AccountAddress::ZERO {
            return None;
        }
        let inner = CoinModule::inner_type(object_type)?;
        Some((inner, owner))
    }
}
