// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::changeset::{ChangeSet, CreatedObject, StateAccessSet};
use crate::common::balance::{
    extract_balance_from_object_bytes, is_balance_struct, token_type_from_struct_tag,
};
use crate::common::ids::canonical_object_id;
use crate::common::keys::{dynamic_field_key, metadata_key, object_key, owned_objects_key};
use crate::storage::object_storage::StoredObject;
use crate::storage::persistent_store::PersistentStore;
use anyhow::{Context, Result, ensure};
use kanari_types::balance::BalanceModule;
use kanari_types::balance::BalanceRecord;
use kanari_types::clock::ClockModule;
use kanari_types::coin::{CoinModule, TreasuryCap};
use kanari_types::error::KanariUnwrapExt;
use kanari_types::event::Event;
use kanari_types::gas_coin::GAS_COIN;
use kanari_types::object::{IDRecord, UIDRecord};
use kanari_types::transaction::ObjectOwnerKind;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{StructTag, TypeTag};
use rocksdb::WriteBatch;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smt;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::str::FromStr;
use std::sync::Arc;

mod access;
mod apply;
mod clock;
mod commit;
mod indexes;
mod objects;
mod owners;
mod roots;
mod supply;

const SYSTEM_CLOCK_OBJECT_ID_KEY: &[u8] = b"system:clock_object_id";
const GENESIS_INITIALIZED_KEY: &[u8] = b"system:genesis_initialized";
const OWNER_INDEX_KEY: &[u8] = b"owner_index";
const LEGACY_ACCOUNT_INDEX_KEY: &[u8] = b"account_index";
const OBJECT_LOCKED_COIN_RECORDS_KEY: &[u8] = b"object_locked_coin_records";
const RUNTIME_STATE_SCHEMA_KEY: &[u8] = b"runtime:state_schema_version";
const RUNTIME_STATE_SCHEMA_VERSION: u32 = 1;
const WALLET_SUPPLY_INDEX_VERSION_KEY: &[u8] = b"runtime:wallet_supply_index_version";
const WALLET_SUPPLY_INDEX_VERSION: u32 = 1;
const ACCESS_VERSIONS_KEY: &[u8] = b"runtime:state_access_versions";
const ACCESS_VERSION_PREFIX: &[u8] = b"runtime:state_access_version:";
const UID_SIZE: usize = 32;
const U64_SIZE: usize = 8;

type RawStateKey = Vec<u8>;
type RawStateValue = Vec<u8>;
type RawStateUpdate = (RawStateKey, RawStateValue);
type RawStateDelete = RawStateKey;
type OverlaySmtChanges = (Vec<RawStateUpdate>, Vec<RawStateDelete>);
pub type PrecomputedSmtChanges = (Vec<(Vec<u8>, Vec<u8>)>, Vec<Vec<u8>>, smt::SmtNodeOverlay);
type DerivedIndexes = (
    Vec<String>,
    BTreeMap<AccountAddress, Vec<String>>,
    Vec<String>,
);

/// Owner state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use]
pub struct OwnerState {
    pub address: AccountAddress,
    pub nonce: u64,
    pub modules: BTreeSet<String>,
    /// Token balances: token_type -> BalanceRecord
    /// Example: "0x840512ff...::james::JAMES" -> BalanceRecord(1000000000)
    pub token_balances: BTreeMap<String, BalanceRecord>,
}

impl OwnerState {
    /// Create a new empty owner state for the given address.
    pub fn new(address: AccountAddress) -> Self {
        Self {
            address,
            nonce: 0,
            modules: BTreeSet::new(),
            token_balances: BTreeMap::new(),
        }
    }

    /// Create an owner state with a pre-set native token balance.
    pub fn with_native_balance(address: AccountAddress, balance: u64) -> Self {
        let mut account = Self::new(address);
        if balance > 0 {
            account.set_token_balance(GAS_COIN.to_string(), BalanceRecord::new(balance));
        }
        account
    }

    fn add_module(&mut self, module_name: String) {
        self.modules.insert(module_name);
    }

    /// Set the balance for a specific token type.
    pub fn set_token_balance(&mut self, token_type: String, amount: BalanceRecord) {
        self.token_balances.insert(token_type, amount);
    }

    fn set_token_balance_value(&mut self, token_type: &str, amount: u64) {
        if amount == 0 {
            self.token_balances.remove(token_type);
        } else {
            self.set_token_balance(token_type.to_string(), BalanceRecord::new(amount));
        }
    }

    /// Get the balance for a specific token type.
    pub fn get_token_balance(&self, token_type: &str) -> u64 {
        self.token_balances
            .get(token_type)
            .map(|b| b.value())
            .unwrap_or(0)
    }

    /// Get the native token balance for this owner.
    pub fn native_balance(&self) -> u64 {
        self.get_token_balance(GAS_COIN)
    }

    /// Get the owner's account address.
    pub fn owner_address(&self) -> AccountAddress {
        self.address
    }

    /// Check if this owner has no modules or token balances.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.token_balances.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenSupplySummary {
    pub token_type: String,
    pub total_supply: u64,
    pub wallet_visible_supply: u64,
    pub object_locked_supply: u64,
    pub accounted_supply: u64,
    pub untracked_supply: u64,
}

/// Read-only diagnostics for the incremental sparse Merkle tree.
///
/// A full audit is intentionally opt-in because it scans canonical state and
/// the persisted SMT leaves. The cheap status path only reads roots, schema
/// versions, and the pending overlay summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmtDiagnostics {
    pub enabled: bool,
    pub persisted_root: Option<String>,
    pub effective_root: String,
    pub overlay_entries: usize,
    pub overlay_updates: usize,
    pub overlay_deletes: usize,
    pub canonical_membership_changed: bool,
    pub runtime_schema_version: Option<u32>,
    pub expected_runtime_schema_version: u32,
    pub wallet_supply_index_version: Option<u32>,
    pub expected_wallet_supply_index_version: u32,
    pub audit_requested: bool,
    pub audit_performed: bool,
    pub persisted_leaf_count: Option<usize>,
    pub consistent: Option<bool>,
    pub consistency_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ObjectLockedCoinRecord {
    pub holder_object_id: String,
    pub holder_type: String,
    pub owner: AccountAddress,
    pub token_type: String,
    pub amount: u64,
}

/// Global state manager for accounts and balances
/// This is a pure data layer that applies ChangeSet from Move VM execution
///
/// Refactored to use RocksDB via PersistentStore for unlimited capacity.
#[derive(Debug, Clone)]
pub struct StateManager {
    pub store: Arc<PersistentStore>,

    /// Overlay for speculative execution / buffering
    /// Key -> Some(Value) or None (Deleted)
    pub overlay: BTreeMap<Vec<u8>, Option<Vec<u8>>>,

    // Cache for total supply to avoid frequent DB reads
    pub total_supply: u64,

    /// In-memory cache for tracking total token supplies in real-time
    pub global_token_supplies: BTreeMap<String, u64>,

    // SMT for state root calculation (Optional: requires DB backend)
    pub smt: Option<Arc<smt::SparseMerkleTree>>,
    pub events: Vec<Event>,

    /// Monotonic versions for deterministic conflict validation. Runtime metadata
    /// is persisted atomically with the overlay but excluded from the state root.
    access_versions: BTreeMap<Vec<u8>, u64>,
    access_epoch: u64,
}

impl StateManager {
    /// Create a new in-memory state manager for testing
    pub fn new_in_memory() -> Self {
        Self::try_new_in_memory().invariant("failed to create in-memory state manager")
    }

    /// Create a new in-memory state manager and surface initialization errors.
    pub(crate) fn try_new_in_memory() -> Result<Self> {
        let store = Arc::new(
            PersistentStore::open_in_memory().context("Failed to create in-memory store")?,
        );
        Self::try_new(store)
    }

    /// Get a clone of the underlying persistent store.
    pub fn store(&self) -> Arc<PersistentStore> {
        self.store.clone()
    }

    /// Create new state with genesis allocation
    /// Total supply: 11 million KANARI = 11,000,000,000,000,000 Mist
    /// Dev address gets entire supply according to kanari.move
    pub fn new(store: Arc<PersistentStore>) -> Self {
        Self::try_new(store).invariant("failed to create state manager")
    }

    /// Create new state with genesis allocation and return initialization errors.
    pub fn try_new(store: Arc<PersistentStore>) -> Result<Self> {
        // Try to load total supply from DB
        let persisted_total_supply_record = store
            .load::<u64>(b"total_supply")
            .context("Failed to load total_supply")?;
        let persisted_total_supply = persisted_total_supply_record.unwrap_or(0);
        let genesis_marker = store
            .load::<bool>(GENESIS_INITIALIZED_KEY)
            .context("Failed to load genesis initialization marker")?
            .unwrap_or(false);

        // Load existing global token supplies from RocksDB
        let global_token_supplies = store
            .load::<BTreeMap<String, u64>>(b"global_token_supplies")
            .context("Failed to load global_token_supplies")?
            .unwrap_or_default();
        let mut access_versions = store
            .load::<BTreeMap<Vec<u8>, u64>>(ACCESS_VERSIONS_KEY)
            .context("Failed to load state access versions")?
            .unwrap_or_default();
        for (key, value) in store
            .logical_entries()
            .context("Failed to load per-key state access versions")?
        {
            let Some(state_key) = key.strip_prefix(ACCESS_VERSION_PREFIX) else {
                continue;
            };
            let version =
                bcs::from_bytes::<u64>(&value).context("Malformed per-key state access version")?;
            access_versions.insert(state_key.to_vec(), version);
        }

        // Recover native total supply from older databases that persisted the
        // native treasury/global balances but never backfilled `total_supply`.
        let recovered_total_supply = if persisted_total_supply == 0 {
            Self::load_persisted_supply_from_store(store.as_ref(), GAS_COIN)?
                .or_else(|| global_token_supplies.get(GAS_COIN).copied())
                .unwrap_or(0)
        } else {
            persisted_total_supply
        };
        let legacy_zero_supply_evidence = if persisted_total_supply_record == Some(0) {
            store
                .logical_entries()
                .context("Failed to inspect legacy state for genesis evidence")?
                .into_iter()
                .any(|(key, _)| {
                    key != b"total_supply"
                        && (key == b"module_index"
                            || key.starts_with(b"module:")
                            || key.starts_with(b"resource:")
                            || key.starts_with(b"account:")
                            || key.starts_with(b"object:")
                            || key.starts_with(b"system:"))
                })
        } else {
            false
        };
        // Presence of the legacy total-supply record is itself evidence that genesis ran,
        // only when other canonical chain state proves this is not a partial/empty DB.
        let genesis_already_initialized =
            genesis_marker || recovered_total_supply > 0 || legacy_zero_supply_evidence;

        // Initialize SMT if store is backed by RocksDB
        let smt = store
            .get_db()
            .map(|db| Arc::new(smt::SparseMerkleTree::new(db)));

        let mut state = Self {
            store,
            overlay: BTreeMap::new(),
            total_supply: recovered_total_supply,
            global_token_supplies,
            smt,
            events: Vec::new(),
            access_versions,
            access_epoch: 0,
        };

        state
            .ensure_smt_initialized()
            .context("Failed to initialize state SMT")?;

        let mut metadata_migration_pending = false;
        if genesis_already_initialized && !genesis_marker {
            state
                .save_internal(GENESIS_INITIALIZED_KEY, &true)
                .context("Failed to stage genesis initialization marker")?;
            metadata_migration_pending = true;
        }

        if persisted_total_supply_record.is_none() && recovered_total_supply > 0 {
            state
                .save_internal(b"total_supply", &recovered_total_supply)
                .context("Failed to backfill recovered total_supply")?;
            metadata_migration_pending = true;
        }

        if metadata_migration_pending {
            state
                .commit()
                .context("Failed to persist state initialization metadata")?;
        }

        // Genesis identity is explicit. Supply is protocol state and may legitimately be zero.
        // The check, initialization, and commit must remain under the same lock: a
        // second initializer can otherwise observe an empty store before waiting and
        // run genesis after the first one has committed.
        if !genesis_already_initialized {
            let genesis_store = state.store.clone();
            let genesis_guard = genesis_store.genesis_init_guard();

            let initialized_while_waiting = state
                .store
                .load::<bool>(GENESIS_INITIALIZED_KEY)
                .context("Failed to re-check genesis initialization marker")?
                .unwrap_or(false);
            if initialized_while_waiting {
                state.total_supply = state
                    .store
                    .load::<u64>(b"total_supply")
                    .context("Failed to reload total_supply after genesis initialization")?
                    .unwrap_or(0);
                state.global_token_supplies = state
                    .store
                    .load::<BTreeMap<String, u64>>(b"global_token_supplies")
                    .context("Failed to reload global token supplies after genesis initialization")?
                    .unwrap_or_default();
            } else {
                state
                    .save_internal(GENESIS_INITIALIZED_KEY, &true)
                    .context("Failed to stage genesis initialization marker")?;
                crate::genesis::init_genesis(&mut state)
                    .context("Genesis initialization failed")?;
                // Flush genesis state to DB immediately while still holding the
                // initialization lock, so subsequent constructors see a complete DB.
                state.commit().context("Failed to commit genesis state")?;
                ensure!(
                    state.total_supply > 0,
                    "Genesis initialization completed but total_supply is still 0"
                );
            }
            drop(genesis_guard);
        }

        state
            .migrate_runtime_state_schema()
            .context("Failed to migrate runtime state schema")?;

        if state
            .repair_derived_indexes_on_startup()
            .context("Failed to rebuild derived indexes on startup")?
        {
            state
                .commit()
                .context("Failed to persist rebuilt derived indexes on startup")?;
        }

        let wallet_supply_index_version = state
            .load_internal::<u32>(WALLET_SUPPLY_INDEX_VERSION_KEY)?
            .unwrap_or(0);
        if wallet_supply_index_version < WALLET_SUPPLY_INDEX_VERSION
            && state
                .repair_legacy_native_wallet_overcount()
                .context("Failed to repair native wallet supply on startup")?
        {
            state
                .commit()
                .context("Failed to persist repaired native wallet supply on startup")?;
        }

        if state
            .ensure_wallet_supply_index()
            .context("Failed to initialize wallet supply index")?
        {
            state
                .commit()
                .context("Failed to persist wallet supply index")?;
        }

        if state
            .ensure_token_holders_index()
            .context("Failed to initialize token holders index")?
        {
            state
                .commit()
                .context("Failed to persist token holders index")?;
        }

        if let Err(e) = state.validate_supply_invariants() {
            Self::report_supply_invariant_violation("on startup", &e)?;
        }

        state
            .ensure_smt_consistent()
            .context("Failed to verify state SMT")?;

        Ok(state)
    }
}

pub(crate) fn default_owner_kind_for_type(
    type_name: &str,
    owner: AccountAddress,
) -> ObjectOwnerKind {
    if type_name.contains("::clock::Clock") {
        ObjectOwnerKind::Shared
    } else if type_name.contains("::coin::CoinMetadata<") {
        ObjectOwnerKind::Immutable
    } else {
        ObjectOwnerKind::AddressOwner(owner.to_hex_literal())
    }
}

#[cfg(test)]
#[path = "../tests/unit/state_tests.rs"]
mod tests;
