// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Transaction changeset types for tracking object mutations and gas accounting.

use hex;
use kanari_crypto::hash_data_blake3;
use kanari_types::coin::TreasuryCap;
use kanari_types::object::IDRecord;
use kanari_types::object::UIDRecord;
use kanari_types::transaction::{
    GasPayment, ObjectChange, ObjectInput, ObjectOwnerKind, ObjectRef,
};
use kanari_types::{balance::BalanceRecord, event::Event};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod changeset_access;
mod changeset_effects;
mod changeset_gas;
mod changeset_merge;
mod changeset_transfers;

/// Created object information captured from Move VM write-sets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use]
pub struct CreatedObject {
    pub owner: AccountAddress,
    pub owner_kind: ObjectOwnerKind,
    /// Optional UIDRecord when object follows UID pattern (for ownership tracking)
    pub uid: Option<UIDRecord>,
    /// Optional IDRecord for DEX/DeFi objects that need copyable IDs
    pub id: Option<IDRecord>,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Vec<u8>,
    pub version: u64,
}

impl CreatedObject {
    /// Returns the owner kind of this object.
    pub fn owner_kind(&self) -> ObjectOwnerKind {
        self.owner_kind.clone()
    }

    /// Returns the hex-encoded Blake3 digest of the object data.
    pub fn digest(&self) -> String {
        format!("0x{}", hex::encode(hash_data_blake3(&self.data)))
    }

    /// Constructs an ObjectRef for this object with the given id, version, and digest.
    pub fn object_ref(&self, object_id: &str) -> ObjectRef {
        ObjectRef::new(
            object_id.to_string(),
            Some(self.version),
            Some(self.digest()),
        )
    }
}

/// Represents owner-state deltas from Move VM execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerDelta {
    pub address: AccountAddress,
    /// Positive = credit, negative = debit. i128 prevents lossy u64 -> i64 casts.
    pub balance_delta: i128,
    pub modules_added: BTreeSet<String>,
}

/// Canonical state keys observed or modified by one transaction execution.
///
/// Keys are deterministic byte strings rather than storage pointers so every
/// validator derives exactly the same conflict decision. The manifest is
/// intentionally conservative: an unclassified write is represented by a
/// dedicated key, which serializes it rather than risking a false negative.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateAccessSet {
    pub reads: BTreeSet<Vec<u8>>,
    pub writes: BTreeSet<Vec<u8>>,
}

impl StateAccessSet {
    /// Returns true if this access set conflicts with another (writes overlap with reads or writes).
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.writes
            .iter()
            .any(|key| other.reads.contains(key) || other.writes.contains(key))
            || other.writes.iter().any(|key| self.reads.contains(key))
    }

    pub(crate) fn read(&mut self, key: impl Into<Vec<u8>>) {
        self.reads.insert(key.into());
    }

    pub(crate) fn write(&mut self, key: impl Into<Vec<u8>>) {
        self.writes.insert(key.into());
    }
}

impl OwnerDelta {
    fn new(address: AccountAddress) -> Self {
        Self {
            address,
            balance_delta: 0,
            modules_added: BTreeSet::new(),
        }
    }

    /// Decrements the balance delta by the given amount.
    pub fn debit(&mut self, amount: u64) {
        self.balance_delta -= amount as i128;
    }

    /// Increments the balance delta by the given amount.
    pub fn credit(&mut self, amount: u64) {
        self.balance_delta += amount as i128;
    }

    pub(crate) fn add_module(&mut self, module_name: String) {
        self.modules_added.insert(module_name);
    }
}

/// ChangeSet represents all state changes from Move VM execution.
/// This is the canonical output from Move VM that StateManager will apply.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[must_use]
pub struct ChangeSet {
    pub owner_deltas: BTreeMap<AccountAddress, OwnerDelta>,
    /// Native gas credits keyed by collector. This is tracked separately from
    /// owner deltas because object-backed balance recomputation must add gas
    /// credits without double-counting transfer or mint deltas already
    /// represented by Coin objects.
    #[serde(default)]
    pub native_gas_credits: BTreeMap<AccountAddress, u64>,
    /// Settlement coin for this transaction's gas (`None` = native KANARI).
    /// Canonical token type, e.g. `0x2::kanari::KANARI`.
    #[serde(default)]
    pub gas_coin_type: Option<String>,
    /// Non-native gas fees keyed by canonical coin type. The sender's gas
    /// object is debited and an equal DAO-owned `Coin<T>` output is recorded in
    /// `created_objects` for the same transaction.
    #[serde(default)]
    pub token_gas_credits: BTreeMap<String, u64>,
    pub events: Vec<Event>,
    /// Treasury creations or updates: (owner, token_type, TreasuryCap)
    pub treasuries: Vec<(AccountAddress, String, TreasuryCap)>,
    /// NFT capability creations or updates: (owner, token_type, NftCapRecord)
    pub nft_caps: Vec<(
        AccountAddress,
        String,
        kanari_types::collection::NftCapRecord,
    )>,
    /// Per-owner token balances (absolute set): (owner, token_type, BalanceRecord)
    pub token_balance_sets: Vec<(AccountAddress, String, BalanceRecord)>,
    pub input_objects: Vec<ObjectInput>,
    pub shared_inputs: Vec<ObjectRef>,
    pub immutable_inputs: Vec<ObjectRef>,
    pub gas_payment: Option<GasPayment>,
    pub gas_object_refs: Vec<ObjectRef>,
    /// Objects created during execution. Each entry is (object_id, CreatedObject)
    pub created_objects: Vec<(String, CreatedObject)>,
    /// Objects deleted during execution. Each entry is object_id
    pub deleted_objects: Vec<String>,
    pub explicit_object_changes: Vec<ObjectChange>,
    /// (object_id, name_bytes, value_bytes)
    pub added_dynamic_fields: Vec<(String, Vec<u8>, Vec<u8>)>,
    /// (object_id, name_bytes)
    pub removed_dynamic_fields: Vec<(String, Vec<u8>)>,
    /// Canonical Move module/resource operations keyed exactly as the shared store.
    /// Some(bytes) is create/modify and None is delete.
    pub move_writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    /// Canonical storage keys read by the Move resolver during this execution.
    /// This is execution metadata and is intentionally not persisted as chain state.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub resolver_reads: BTreeSet<Vec<u8>>,
    pub gas_used: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl ChangeSet {
    fn with_status(gas_used: u64, success: bool, error_message: Option<String>) -> Self {
        Self {
            owner_deltas: BTreeMap::new(),
            native_gas_credits: BTreeMap::new(),
            gas_coin_type: None,
            token_gas_credits: BTreeMap::new(),
            events: Vec::new(),
            treasuries: Vec::new(),
            nft_caps: Vec::new(),
            token_balance_sets: Vec::new(),
            input_objects: Vec::new(),
            shared_inputs: Vec::new(),
            immutable_inputs: Vec::new(),
            gas_payment: None,
            gas_object_refs: Vec::new(),
            created_objects: Vec::new(),
            deleted_objects: Vec::new(),
            explicit_object_changes: Vec::new(),
            added_dynamic_fields: Vec::new(),
            removed_dynamic_fields: Vec::new(),
            move_writes: BTreeMap::new(),
            resolver_reads: BTreeSet::new(),
            gas_used,
            success,
            error_message,
        }
    }

    /// Creates a new empty ChangeSet with default success status.
    pub fn new() -> Self {
        Self::with_status(0, true, None)
    }

    /// Returns a mutable reference to the owner delta for the given address, creating one if absent.
    pub fn get_or_create_owner_delta(&mut self, address: AccountAddress) -> &mut OwnerDelta {
        self.owner_deltas
            .entry(address)
            .or_insert_with(|| OwnerDelta::new(address))
    }

    /// Returns true if this ChangeSet contains no side effects.
    pub fn is_empty(&self) -> bool {
        self.owner_deltas.is_empty()
            && self.native_gas_credits.is_empty()
            && self.gas_coin_type.is_none()
            && self.token_gas_credits.is_empty()
            && self.events.is_empty()
            && self.treasuries.is_empty()
            && self.token_balance_sets.is_empty()
            && self.input_objects.is_empty()
            && self.shared_inputs.is_empty()
            && self.immutable_inputs.is_empty()
            && self.gas_payment.is_none()
            && self.gas_object_refs.is_empty()
            && self.created_objects.is_empty()
            && self.deleted_objects.is_empty()
            && self.explicit_object_changes.is_empty()
            && self.added_dynamic_fields.is_empty()
            && self.removed_dynamic_fields.is_empty()
            && self.move_writes.is_empty()
            && self.gas_used == 0
            && self.success
            && self.error_message.is_none()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/changeset_tests.rs"]
mod tests;
