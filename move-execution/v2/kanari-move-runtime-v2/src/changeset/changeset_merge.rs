// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Merging changesets (by value and by reference).

use crate::changeset::ChangeSet;

impl ChangeSet {
    /// Merge another owned ChangeSet into this one. Later Move writes replace earlier
    /// writes for the same canonical key, matching serial transaction execution semantics.
    ///
    /// `other` is taken by value so the merge can reuse its vector allocations instead
    /// of copying them; callers that still need `other` should use [`ChangeSet::merge_from`].
    /// The two are semantically identical — a change to one must be mirrored in the other,
    /// which `merge_and_merge_from_produce_identical_changesets` guards.
    pub fn merge(&mut self, mut other: ChangeSet) {
        for (addr, other_change) in other.owner_deltas {
            let existing = self.get_or_create_owner_delta(addr);
            existing.balance_delta = existing
                .balance_delta
                .saturating_add(other_change.balance_delta);
            existing.modules_added.extend(other_change.modules_added);
        }
        for (collector, amount) in other.native_gas_credits {
            let collected = self.native_gas_credits.entry(collector).or_insert(0);
            *collected = collected.saturating_add(amount);
        }
        if self.gas_coin_type.is_none() {
            self.gas_coin_type = other.gas_coin_type;
        }
        for (coin_type, amount) in other.token_gas_credits {
            let collected = self.token_gas_credits.entry(coin_type).or_insert(0);
            *collected = collected.saturating_add(amount);
        }
        self.events.extend(other.events);
        self.treasuries.extend(other.treasuries);
        self.nft_caps.extend(other.nft_caps);
        self.input_objects.append(&mut other.input_objects);
        self.shared_inputs.append(&mut other.shared_inputs);
        self.immutable_inputs.append(&mut other.immutable_inputs);
        if self.gas_payment.is_none() {
            self.gas_payment = other.gas_payment.take();
        }
        self.gas_object_refs.append(&mut other.gas_object_refs);

        for (owner, token_type, amount) in other.token_balance_sets {
            self.add_token_balance_set(owner, token_type, amount.value());
        }

        self.created_objects.extend(other.created_objects);
        self.deleted_objects.extend(other.deleted_objects);
        self.explicit_object_changes
            .append(&mut other.explicit_object_changes);
        self.added_dynamic_fields
            .append(&mut other.added_dynamic_fields);
        self.removed_dynamic_fields
            .append(&mut other.removed_dynamic_fields);
        for (key, value) in other.move_writes {
            self.move_writes.insert(key, value);
        }
        self.resolver_reads.append(&mut other.resolver_reads);

        self.gas_used = self.gas_used.saturating_add(other.gas_used);
        if !other.success {
            self.success = false;
            self.error_message = other.error_message;
        }
    }

    /// Merge another ChangeSet by reference.
    ///
    /// This is [`ChangeSet::merge`] with the ownership flipped: high-throughput
    /// checkpoint preparation keeps the source set alive instead of cloning an entire
    /// vector of transaction effects before the merged batch is built.
    /// Later Move writes still replace earlier writes for the same canonical key.
    pub fn merge_from(&mut self, other: &ChangeSet) {
        self.events.reserve(other.events.len());
        self.treasuries.reserve(other.treasuries.len());
        self.nft_caps.reserve(other.nft_caps.len());
        self.input_objects.reserve(other.input_objects.len());
        self.shared_inputs.reserve(other.shared_inputs.len());
        self.immutable_inputs.reserve(other.immutable_inputs.len());
        self.gas_object_refs.reserve(other.gas_object_refs.len());
        self.created_objects.reserve(other.created_objects.len());
        self.deleted_objects.reserve(other.deleted_objects.len());
        self.explicit_object_changes
            .reserve(other.explicit_object_changes.len());
        self.added_dynamic_fields
            .reserve(other.added_dynamic_fields.len());
        self.removed_dynamic_fields
            .reserve(other.removed_dynamic_fields.len());

        for (addr, other_change) in &other.owner_deltas {
            let existing = self.get_or_create_owner_delta(*addr);
            existing.balance_delta = existing
                .balance_delta
                .saturating_add(other_change.balance_delta);
            existing
                .modules_added
                .extend(other_change.modules_added.iter().cloned());
        }
        for (collector, amount) in &other.native_gas_credits {
            let collected = self.native_gas_credits.entry(*collector).or_insert(0);
            *collected = collected.saturating_add(*amount);
        }
        if self.gas_coin_type.is_none() {
            self.gas_coin_type = other.gas_coin_type.clone();
        }
        for (coin_type, amount) in &other.token_gas_credits {
            let collected = self.token_gas_credits.entry(coin_type.clone()).or_insert(0);
            *collected = collected.saturating_add(*amount);
        }
        self.events.extend(other.events.iter().cloned());
        self.treasuries.extend(other.treasuries.iter().cloned());
        self.nft_caps.extend(other.nft_caps.iter().cloned());
        self.input_objects
            .extend(other.input_objects.iter().cloned());
        self.shared_inputs
            .extend(other.shared_inputs.iter().cloned());
        self.immutable_inputs
            .extend(other.immutable_inputs.iter().cloned());
        if self.gas_payment.is_none() {
            self.gas_payment = other.gas_payment.clone();
        }
        self.gas_object_refs
            .extend(other.gas_object_refs.iter().cloned());

        for (owner, token_type, amount) in &other.token_balance_sets {
            self.add_token_balance_set(*owner, token_type.clone(), amount.value());
        }

        self.created_objects
            .extend(other.created_objects.iter().cloned());
        self.deleted_objects
            .extend(other.deleted_objects.iter().cloned());
        self.explicit_object_changes
            .extend(other.explicit_object_changes.iter().cloned());
        self.added_dynamic_fields
            .extend(other.added_dynamic_fields.iter().cloned());
        self.removed_dynamic_fields
            .extend(other.removed_dynamic_fields.iter().cloned());
        for (key, value) in &other.move_writes {
            self.move_writes.insert(key.clone(), value.clone());
        }
        self.resolver_reads
            .extend(other.resolver_reads.iter().cloned());

        self.gas_used = self.gas_used.saturating_add(other.gas_used);
        if !other.success {
            self.success = false;
            self.error_message = other.error_message.clone();
        }
    }
}
