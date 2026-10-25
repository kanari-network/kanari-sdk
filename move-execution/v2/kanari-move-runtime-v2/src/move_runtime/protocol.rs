// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Consensus-critical asset transfer policy gates.
//!
//! These gates intentionally live outside the entry execution pipeline so
//! reviewers can audit all deny/pause enforcement in one place. Call sites
//! remain in entry.rs: before parameter binding and before persistence.

use super::*;
use kanari_types::{coin::CoinModule, krc_721::Krc721Module};

/// Gas budget for one framework deny-policy read inside a consensus gate.
pub(crate) const DENY_POLICY_GAS_BUDGET: u64 = 1_000_000;

impl MoveRuntime {
    // Create a new Move session with the given resolver and a set of extensions
    pub(crate) fn create_session_with_resolver<'r>(
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

    /// Output-policy candidate: a coin-type object landing at a non-zero
    /// address owner.
    pub(crate) fn coin_recipient_candidate(
        object_type: &str,
        owner: AccountAddress,
    ) -> Option<(String, AccountAddress)> {
        if owner == AccountAddress::ZERO {
            return None;
        }
        let inner = CoinModule::inner_type(object_type)?;
        Some((inner, owner))
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

    /// Read a non-generic KRC-721 deny-list predicate.  Like the generic
    /// coin reader above, a failed read is deliberately returned as `None` so
    /// every consensus caller can fail closed with context appropriate to its
    /// phase (input or output).
    fn read_krc721_deny_flag(
        session: &mut move_vm_runtime::session::Session<
            crate::storage::resolver::KanariMoveResolver,
        >,
        function_name: &str,
        args: Vec<Vec<u8>>,
        gas: &mut crate::kanari_gas_meter::KanariGasMeter,
        collection: AccountAddress,
    ) -> Result<Option<bool>> {
        let ident = IdentStr::new(function_name)?;
        match session.execute_function_bypass_visibility(
            &Krc721Module::module_id(),
            ident,
            vec![],
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
                    "[RUNTIME] KRC-721 deny policy read {function_name} failed for collection {}: {error:?}",
                    collection.to_hex_literal(),
                );
                Ok(None)
            }
        }
    }

    /// Return the collection carried by a canonical KRC-721 object, or a
    /// fail-closed error when an object claiming that type has invalid BCS.
    fn krc721_collection_candidate(
        object_type: &str,
        data: &[u8],
    ) -> Result<Option<AccountAddress>> {
        if !Krc721Module::is_nft_object_type(object_type) {
            return Ok(None);
        }
        Krc721Module::collection_from_nft_bcs(data).map(Some).ok_or_else(|| {
            anyhow::anyhow!(
                "KRC-721 policy: malformed canonical NFT data; refusing to bypass collection deny policy"
            )
        })
    }

    /// Consensus gate for KRC-721 inputs.
    ///
    /// KRC-721 NFTs intentionally retain `store`, allowing wallet and
    /// marketplace composition.  That also means Move's public-transfer
    /// native is callable without the KRC wrapper.  This gate examines every
    /// declared and raw object reference before the entry is bound and rejects
    /// a denied sender or a paused collection irrespective of the called Move
    /// function.  Missing/corrupt policy state rejects the transaction.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn enforce_krc721_input_policy(
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
        if bypass_entry_check {
            return Ok(());
        }
        let Some(sender) = sender else {
            return Ok(());
        };

        let mut collections = Vec::new();
        for input in object_inputs {
            if let Some(stored) =
                self.get_object_for_execution(&input.object_ref.object_id, overlay)?
                && let Some(collection) =
                    Self::krc721_collection_candidate(&stored.type_name, &stored.data)?
            {
                collections.push(collection);
            }
        }
        for arg in raw_args {
            if arg.len() != AccountAddress::LENGTH {
                continue;
            }
            let object_id = format!("0x{}", hex::encode(arg));
            if let Some(stored) = self.get_object_for_execution(&object_id, overlay)?
                && let Some(collection) =
                    Self::krc721_collection_candidate(&stored.type_name, &stored.data)?
            {
                collections.push(collection);
            }
        }
        collections.sort();
        collections.dedup();
        if collections.is_empty() {
            return Ok(());
        }

        let Some(singleton) =
            self.get_object_for_execution(&Self::deny_list_singleton_full_id(), overlay)?
        else {
            anyhow::bail!(
                "KRC-721 policy: shared deny-list singleton {} is missing; refusing NFT movement",
                Self::deny_list_singleton_full_id(),
            );
        };
        let sender_bytes = bcs::to_bytes(&sender)?;
        let mut policy_gas = crate::kanari_gas_meter::KanariGasMeter::new(DENY_POLICY_GAS_BUDGET);
        for collection in collections {
            let collection_bytes = bcs::to_bytes(&collection)?;
            let paused = Self::read_krc721_deny_flag(
                session,
                Krc721Module::function_names().is_paused,
                vec![singleton.data.clone(), collection_bytes.clone()],
                &mut policy_gas,
                collection,
            )?.ok_or_else(|| anyhow::anyhow!(
                "KRC-721 policy: pause state for collection {} could not be read; refusing transaction (fail-closed)",
                collection.to_hex_literal(),
            ))?;
            if paused {
                anyhow::bail!(
                    "transaction sender {} is blocked: KRC-721 collection {} is globally paused (deny-list policy)",
                    sender.to_hex_literal(),
                    collection.to_hex_literal(),
                );
            }
            let denied = Self::read_krc721_deny_flag(
                session,
                Krc721Module::function_names().is_denied,
                vec![singleton.data.clone(), collection_bytes, sender_bytes.clone()],
                &mut policy_gas,
                collection,
            )?.ok_or_else(|| anyhow::anyhow!(
                "KRC-721 policy: sender deny state for collection {} could not be read; refusing transaction (fail-closed)",
                collection.to_hex_literal(),
            ))?;
            if denied {
                anyhow::bail!(
                    "transaction sender {} is denied for KRC-721 collection {} (deny-list policy)",
                    sender.to_hex_literal(),
                    collection.to_hex_literal(),
                );
            }
        }
        Ok(())
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
    pub(crate) fn enforce_coin_output_policy(
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

    /// Consensus gate for KRC-721 outputs.
    ///
    /// Checks every final address owner produced by a KRC-721 transfer,
    /// including a direct `transfer::public_transfer` call.  It runs before
    /// `add_transferred_objects_to_changeset`, so a denied recipient or a
    /// collection paused during execution cannot receive the NFT and no
    /// partial state is persisted.
    pub(crate) fn enforce_krc721_output_policy(
        &self,
        vm_guard: &std::sync::RwLockReadGuard<'_, MoveVM>,
        _cs: &ChangeSet,
        transferred: &[TransferredObject],
        state_overlay: Option<crate::StateOverlay>,
    ) -> Result<()> {
        // Unlike fungible coins, a freshly minted KRC-721 object is not a
        // transfer.  Restrict this scan to transfer-native records: that
        // closes `public_transfer` while preserving the issuer's explicit
        // authority to mint (including minting a replacement after a burn).
        let mut targets: Vec<(AccountAddress, AccountAddress)> = Vec::new();
        for object in transferred
            .iter()
            .filter(|object| object.should_persist && !object.is_frozen && !object.is_shared)
        {
            if object.recipient == AccountAddress::ZERO {
                continue;
            }
            if let Some(collection) =
                Self::krc721_collection_candidate(&object.object_type, &object.data)?
            {
                targets.push((collection, object.recipient));
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
                "KRC-721 output policy: shared deny-list singleton {} is missing; refusing NFT delivery",
                Self::deny_list_singleton_full_id(),
            );
        };
        let (resolver, _resolver_trace) = self
            .resolver
            .tracing_clone_with_overlay(state_overlay.clone());
        let mut session = self.create_session_with_resolver(vm_guard, resolver, state_overlay);
        let mut policy_gas = crate::kanari_gas_meter::KanariGasMeter::new(DENY_POLICY_GAS_BUDGET);
        for (collection, recipient) in targets {
            let collection_bytes = bcs::to_bytes(&collection)?;
            let paused = Self::read_krc721_deny_flag(
                &mut session,
                Krc721Module::function_names().is_paused,
                vec![singleton.data.clone(), collection_bytes.clone()],
                &mut policy_gas,
                collection,
            )?.ok_or_else(|| anyhow::anyhow!(
                "KRC-721 output policy: pause state for collection {} could not be read; refusing transaction (fail-closed)",
                collection.to_hex_literal(),
            ))?;
            if paused {
                anyhow::bail!(
                    "recipient {} cannot receive from KRC-721 collection {}: it is globally paused (deny-list policy)",
                    recipient.to_hex_literal(),
                    collection.to_hex_literal(),
                );
            }
            let denied = Self::read_krc721_deny_flag(
                &mut session,
                Krc721Module::function_names().is_denied,
                vec![singleton.data.clone(), collection_bytes, bcs::to_bytes(&recipient)?],
                &mut policy_gas,
                collection,
            )?.ok_or_else(|| anyhow::anyhow!(
                "KRC-721 output policy: recipient deny state for collection {} could not be read; refusing transaction (fail-closed)",
                collection.to_hex_literal(),
            ))?;
            if denied {
                anyhow::bail!(
                    "recipient {} is denied for KRC-721 collection {} (deny-list policy)",
                    recipient.to_hex_literal(),
                    collection.to_hex_literal(),
                );
            }
        }
        Ok(())
    }
}
