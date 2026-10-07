// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Deterministic access manifests for conflict validation.

use crate::changeset::{ChangeSet, OwnerDelta, StateAccessSet};
use move_core_types::account_address::AccountAddress;

impl ChangeSet {
    /// Produce a deterministic, conservative access manifest from canonical VM
    /// effects. Resolver read tracing is added separately; this manifest already
    /// covers every object, owner, resource, dynamic field and gas write that
    /// reaches StateManager.
    pub fn deterministic_access_set(&self) -> StateAccessSet {
        let mut access = StateAccessSet::default();

        access.reads.extend(self.resolver_reads.iter().cloned());
        // Dynamic-field natives use a separate resolver extension. Until that
        // extension exposes its precise read key, serialize only transactions
        // that write a dynamic field against this conservative read fence.
        access.read(b"df:*".to_vec());

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
            access.write(format!(
                "token_balance:{}:{}",
                owner.to_hex_literal(),
                token_type
            ));
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
        for (object_id, name, _) in &self.added_dynamic_fields {
            access.write(b"df:*".to_vec());
            access.write(Self::dynamic_field_access_key(object_id, name));
        }
        for (object_id, name) in &self.removed_dynamic_fields {
            access.write(b"df:*".to_vec());
            access.write(Self::dynamic_field_access_key(object_id, name));
        }
        for key in self.move_writes.keys() {
            access.write(key.clone());
        }
        for event in &self.events {
            if event.type_tag.to_string().contains("::nft::MintLog") && event.event_data.len() >= 96
            {
                access.write(format!(
                    "collection_members:0x{}",
                    hex::encode(&event.event_data[64..96])
                ));
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
        let Ok(payment_owner) = AccountAddress::from_hex_literal(&payment.owner) else {
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
        AccountAddress::from_hex_literal(object_id)
            .map(|addr| addr.to_hex_literal())
            .unwrap_or_else(|_| object_id.to_string())
    }

    /// Deterministic-access key for one object record.
    pub(crate) fn object_access_key(object_id: &str) -> String {
        format!("object:{}", Self::canonicalize_object_id(object_id))
    }

    /// Deterministic-access key for one dynamic-field record.
    pub(crate) fn dynamic_field_access_key(object_id: &str, name: &[u8]) -> String {
        format!(
            "dynamic:{}:{}",
            Self::canonicalize_object_id(object_id),
            hex::encode(name)
        )
    }
}
