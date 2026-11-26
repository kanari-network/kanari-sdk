// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Deterministic access manifests for conflict validation.

use crate::changeset::{ChangeSet, OwnerDelta, StateAccessSet};
use kanari_types::collection::CollectionModule;
use move_core_types::account_address::AccountAddress;

impl ChangeSet {
    /// Produce a deterministic, conservative access manifest from canonical VM
    /// effects. Resolver read tracing is added separately; this manifest already
    /// covers every object, owner, resource, dynamic field and gas write that
    /// reaches StateManager.
    pub fn deterministic_access_set(&self) -> StateAccessSet {
        let mut access = StateAccessSet::default();

        access.reads.extend(self.resolver_reads.iter().cloned());
        // Entry execution traces every dynamic-field native read into
        // `resolver_reads`, so only legacy/untraced changesets still need the
        // coarse reader fence. `propagate_untraced_dynamic_field_fence` adds the
        // matching writer fence to dynamic-field writers when a batch mixes
        // traced and untraced transactions.
        if !self.dynamic_field_reads_traced {
            access.read(untraced_dynamic_field_fence());
        }

        for input in &self.input_objects {
            access.read(Self::object_access_key(&input.object_ref.object_id));
        }
        for input in &self.shared_inputs {
            access.read(Self::object_access_key(&input.object_id));
        }
        for input in &self.immutable_inputs {
            access.read(Self::object_access_key(&input.object_id));
        }
        for input in &self.gas_object_refs {
            access.write(Self::object_access_key(&input.object_id));
        }
        if let Some(payment) = &self.gas_payment {
            for input in &payment.payment_objects {
                access.write(Self::object_access_key(&input.object_id));
            }
        }
        for (owner, delta) in &self.owner_deltas {
            if self.is_parallel_safe_native_owner_delta(owner, delta) {
                continue;
            }
            access.write(format!("owner:{}", owner.to_hex_literal()));
        }
        for (_, token_type, _) in &self.treasuries {
            access.write(format!("supply:{token_type}"));
            access.write(format!("treasury:{token_type}"));
            access.write(b"global_token_supplies".to_vec());
        }
        for (_, token_type, _) in &self.nft_caps {
            access.write(format!("nft:{token_type}"));
        }
        for (owner, token_type, _) in &self.token_balance_sets {
            access.write(format!("token_balance:{}:{}", owner.to_hex_literal(), token_type));
        }
        for (object_id, _) in &self.created_objects {
            access.write(Self::object_access_key(object_id));
        }
        for object_id in &self.deleted_objects {
            access.write(Self::object_access_key(object_id));
        }
        for change in &self.explicit_object_changes {
            access.write(Self::object_access_key(&change.object_ref.object_id));
        }
        // An untraced writer still publishes the historical coarse write fence,
        // but only when it actually writes a dynamic field. Read-only changesets
        // keep the old read-only fence so read/read remains conflict-free.
        if !self.dynamic_field_reads_traced
            && (!self.added_dynamic_fields.is_empty() || !self.removed_dynamic_fields.is_empty())
        {
            access.write(untraced_dynamic_field_fence());
        }
        for (object_id, name, _) in &self.added_dynamic_fields {
            access.writes.extend(Self::dynamic_field_access_key(object_id, name));
        }
        for (object_id, name) in &self.removed_dynamic_fields {
            access.writes.extend(Self::dynamic_field_access_key(object_id, name));
        }
        for key in self.move_writes.keys() {
            access.write(key.clone());
        }
        for event in &self.events {
            if CollectionModule::is_mint_log_type(&event.type_tag)
                && let Some((_, collection_id)) = CollectionModule::mint_log_ids(&event.event_data)
            {
                access
                    .write(format!("collection_members:0x{}", hex::encode(collection_id.as_ref())));
            }
        }
        access
    }

    fn is_parallel_safe_native_owner_delta(
        &self,
        owner: &AccountAddress,
        delta: &OwnerDelta,
    ) -> bool {
        if !delta.modules_added.is_empty() {
            return false;
        }
        if delta.balance_delta >= 0 {
            return false;
        }

        let Some(payment) = &self.gas_payment else {
            return false;
        };
        if payment.payment_objects.is_empty() {
            return false;
        }
        let Some(payment_owner) = payment.owner_account_strict() else {
            return false;
        };
        payment_owner == *owner
    }

    /// Records resolver read keys into the access set.
    pub fn record_resolver_reads<I>(&mut self, reads: I)
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        self.resolver_reads.extend(reads);
    }

    pub(crate) fn canonicalize_object_id(object_id: &str) -> String {
        crate::common::ids::canonical_object_id_invariant(object_id)
    }

    /// Deterministic-access key for one object record.
    pub(crate) fn object_access_key(object_id: &str) -> String {
        format!("object:{}", Self::canonicalize_object_id(object_id))
    }

    /// Deterministic-access key for one dynamic-field record. This is the exact
    /// storage key used by both the dynamic-field natives and `apply_dynamic_fields`,
    /// so precise traced reads and writes compare equal.
    pub(crate) fn dynamic_field_access_key(object_id: &str, name: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = vec![crate::common::keys::dynamic_field_key(object_id, name)];
        if let Some(canonical) = Self::canonical_object_id(object_id) {
            let canonical_key = crate::common::keys::dynamic_field_key(&canonical, name);
            if canonical_key != keys[0] {
                keys.push(canonical_key);
            }
        }
        keys
    }

    fn canonical_object_id(object_id: &str) -> Option<String> {
        crate::common::ids::canonical_object_id(object_id)
    }
}

/// Coarse read/write fence used only for executions that cannot trace dynamic-field
/// reads (legacy changesets, publish/init sessions, deserialized metadata).
pub fn untraced_dynamic_field_fence() -> Vec<u8> {
    b"df:*".to_vec()
}

/// Derive access manifests for a whole batch and add the writer side of the
/// dynamic-field fence when any member is untraced. Exact-field writers then
/// conflict with an untraced reader without forcing read-only changesets to
/// conflict with unrelated dynamic-field writes.
pub fn deterministic_access_sets<'a, I>(changesets: I) -> Vec<StateAccessSet>
where
    I: IntoIterator<Item = &'a ChangeSet>,
{
    let changesets = changesets.into_iter().collect::<Vec<_>>();
    let writes_dynamic_field = changesets
        .iter()
        .map(|cs| !cs.added_dynamic_fields.is_empty() || !cs.removed_dynamic_fields.is_empty())
        .collect::<Vec<_>>();
    let mut sets = changesets.iter().map(|cs| cs.deterministic_access_set()).collect::<Vec<_>>();
    propagate_untraced_dynamic_field_fence(&mut sets, &writes_dynamic_field);
    sets
}

/// Add the coarse writer fence only to dynamic-field writers in a batch that
/// contains an untraced reader. Read-only changesets keep their read-only fence
/// so read/read parallelism is preserved.
pub fn propagate_untraced_dynamic_field_fence(
    sets: &mut [StateAccessSet],
    writes_dynamic_field: &[bool],
) {
    let fence = untraced_dynamic_field_fence();
    if sets.iter().any(|set| set.reads.contains(&fence)) {
        for (set, writes_df) in sets.iter_mut().zip(writes_dynamic_field) {
            if *writes_df {
                set.writes.insert(fence.clone());
            }
        }
    }
}
