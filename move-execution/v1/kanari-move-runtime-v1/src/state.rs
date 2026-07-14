// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::changeset::{ChangeSet, CreatedObject};
use crate::common::ids::canonical_object_id;
use crate::common::keys::{metadata_key, object_key, owned_objects_key};
use crate::storage::object_storage::StoredObject;
use crate::storage::persistent_store::PersistentStore;
use anyhow::{Context, Result, ensure};
use kanari_crypto::hash_data_blake3;
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
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smt;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::str::FromStr;
use std::sync::Arc;

mod apply;
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
const UID_SIZE: usize = 32;
const U64_SIZE: usize = 8;

type RawStateKey = Vec<u8>;
type RawStateValue = Vec<u8>;
type RawStateUpdate = (RawStateKey, RawStateValue);
type RawStateDelete = RawStateKey;
type OverlaySmtChanges = (Vec<RawStateUpdate>, Vec<RawStateDelete>);
type DerivedIndexes = (
    Vec<String>,
    BTreeMap<AccountAddress, Vec<String>>,
    Vec<String>,
);

/// Owner state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerState {
    pub address: AccountAddress,
    pub nonce: u64,
    pub modules: BTreeSet<String>,
    /// Token balances: token_type -> BalanceRecord
    /// Example: "0x840512ff...::james::JAMES" -> BalanceRecord(1000000000)
    pub token_balances: BTreeMap<String, BalanceRecord>,
}

impl OwnerState {
    pub fn new(address: AccountAddress) -> Self {
        Self {
            address,
            nonce: 0,
            modules: BTreeSet::new(),
            token_balances: BTreeMap::new(),
        }
    }

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

    pub fn get_token_balance(&self, token_type: &str) -> u64 {
        self.token_balances
            .get(token_type)
            .map(|b| b.value())
            .unwrap_or(0)
    }

    pub fn native_balance(&self) -> u64 {
        self.get_token_balance(GAS_COIN)
    }

    pub fn owner_address(&self) -> AccountAddress {
        self.address
    }

    pub fn owned_token_balances(&self) -> &BTreeMap<String, BalanceRecord> {
        &self.token_balances
    }

    pub fn increment_sequence(&mut self) {
        // Legacy no-op. Owner/account sequence is not part of Kanari execution semantics.
    }

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
struct ObjectLockedCoinRecord {
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
}

impl StateManager {
    fn collect_derived_indexes(&self) -> Result<DerivedIndexes> {
        let entries = self
            .store
            .logical_entries()
            .context("Failed to read state entries for derived index rebuild")?;

        let mut owner_ids = BTreeSet::new();
        let mut owned_objects: BTreeMap<AccountAddress, BTreeSet<String>> = BTreeMap::new();
        let mut object_ids = BTreeSet::new();

        for (key, value) in entries {
            if let Some(owner_bytes) = key.strip_prefix(b"account:") {
                if let Ok(owner) = AccountAddress::from_bytes(owner_bytes) {
                    owner_ids.insert(owner.to_hex_literal());
                }
                continue;
            }

            let Some(object_id_bytes) = key.strip_prefix(b"object:") else {
                continue;
            };
            let Ok(mut object_id) = String::from_utf8(object_id_bytes.to_vec()) else {
                continue;
            };
            if let Some(canonical_id) = canonical_object_id(&object_id) {
                object_id = canonical_id;
            }
            object_ids.insert(object_id.clone());

            let Ok(stored) = bcs::from_bytes::<StoredObject>(&value) else {
                continue;
            };
            if matches!(stored.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                owner_ids.insert(stored.owner.to_hex_literal());
                owned_objects
                    .entry(stored.owner)
                    .or_default()
                    .insert(object_id);
            }
        }

        Ok((
            owner_ids.into_iter().collect(),
            owned_objects
                .into_iter()
                .map(|(owner, ids)| (owner, ids.into_iter().collect()))
                .collect(),
            object_ids.into_iter().collect(),
        ))
    }

    fn load_index_list(&self, key: &[u8]) -> Result<Vec<String>> {
        Ok(self.load_internal(key)?.unwrap_or_default())
    }

    fn save_index_list(&mut self, key: &[u8], ids: &[String]) -> Result<()> {
        self.save_internal(key, ids)
    }

    fn add_to_index_list(&mut self, key: &[u8], value: String) -> Result<()> {
        let mut index = self.load_index_list(key)?;
        let Err(pos) = index.binary_search(&value) else {
            return Ok(());
        };

        index.insert(pos, value);
        self.save_index_list(key, &index)?;
        Ok(())
    }

    fn remove_from_index_list(&mut self, key: &[u8], value: &str) -> Result<()> {
        let mut index = self.load_index_list(key)?;
        let Ok(pos) = index.binary_search_by(|entry| entry.as_str().cmp(value)) else {
            return Ok(());
        };

        index.remove(pos);
        self.save_index_list(key, &index)?;
        Ok(())
    }

    /// Retrieves all Collection IDs from the index
    pub fn get_all_collection_ids(&self) -> Vec<String> {
        self.load_index_list(b"nft_collection_index")
            .unwrap_or_default()
    }

    /// Retrieves NFT IDs for the specified collection from the index
    pub fn get_collection_nft_ids(&self, collection_id: &str) -> Vec<String> {
        self.load_index_list(&metadata_key(b"collection_members:", collection_id))
            .unwrap_or_default()
    }

    fn is_balance_struct(struct_tag: &StructTag) -> bool {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();
        (module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT)
            || (module_name == BalanceModule::BALANCE_MODULE
                && struct_name == BalanceModule::BALANCE_STRUCT)
    }

    fn extract_balance_from_object_bytes(data: &[u8], struct_tag: &StructTag) -> Option<u64> {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();

        if module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT {
            if data.len() < UID_SIZE + U64_SIZE {
                return None;
            }
            let bytes: [u8; U64_SIZE] = data[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
            return Some(u64::from_le_bytes(bytes));
        }

        if module_name == BalanceModule::BALANCE_MODULE
            && struct_name == BalanceModule::BALANCE_STRUCT
        {
            if data.len() < U64_SIZE {
                return None;
            }
            let bytes: [u8; U64_SIZE] = data[data.len() - U64_SIZE..].try_into().ok()?;
            return Some(u64::from_le_bytes(bytes));
        }

        None
    }

    fn write_balance_to_object_bytes(data: &mut [u8], struct_tag: &StructTag, amount: u64) -> bool {
        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();
        let bytes = amount.to_le_bytes();

        if module_name == CoinModule::COIN_MODULE && struct_name == CoinModule::COIN_STRUCT {
            if data.len() < UID_SIZE + U64_SIZE {
                return false;
            }
            data[UID_SIZE..(UID_SIZE + U64_SIZE)].copy_from_slice(&bytes);
            return true;
        }

        if module_name == BalanceModule::BALANCE_MODULE
            && struct_name == BalanceModule::BALANCE_STRUCT
        {
            if data.len() < U64_SIZE {
                return false;
            }
            let start = data.len() - U64_SIZE;
            data[start..].copy_from_slice(&bytes);
            return true;
        }

        false
    }
    fn token_type_from_balance_struct(struct_tag: &StructTag) -> Option<String> {
        if let Some(TypeTag::Struct(st)) = struct_tag.type_params.first() {
            return Some(format!("{}", st));
        }
        None
    }

    fn treasury_cap_token_supply(type_name: &str, data: &[u8]) -> Option<(String, u64)> {
        let struct_tag = StructTag::from_str(type_name).ok()?;
        if struct_tag.module.as_str() != CoinModule::COIN_MODULE
            || struct_tag.name.as_str() != CoinModule::TREASURY_CAP_STRUCT
        {
            return None;
        }
        let token_type = Self::token_type_from_balance_struct(&struct_tag)?;
        if data.len() < UID_SIZE + U64_SIZE {
            return None;
        }
        let supply_bytes: [u8; U64_SIZE] = data[UID_SIZE..(UID_SIZE + U64_SIZE)].try_into().ok()?;
        Some((
            Self::normalize_token_type(&token_type),
            u64::from_le_bytes(supply_bytes),
        ))
    }

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

        // Recover native total supply from older databases that persisted the
        // native treasury/global balances but never backfilled `total_supply`.
        let recovered_total_supply = if persisted_total_supply == 0 {
            Self::load_persisted_supply_from_store(store.as_ref(), GAS_COIN)
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
        if !genesis_already_initialized {
            state
                .save_internal(GENESIS_INITIALIZED_KEY, &true)
                .context("Failed to stage genesis initialization marker")?;
            crate::genesis::init_genesis(&mut state).context("Genesis initialization failed")?;
            // Flush genesis state to DB immediately
            state.commit().context("Failed to commit genesis state")?;
            ensure!(
                state.total_supply > 0,
                "Genesis initialization completed but total_supply is still 0"
            );
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

        if let Err(e) = state.validate_supply_invariants() {
            Self::report_supply_invariant_violation("on startup", &e)?;
        }

        state
            .ensure_smt_consistent()
            .context("Failed to verify state SMT")?;

        Ok(state)
    }

    fn migrate_runtime_state_schema(&mut self) -> Result<()> {
        let persisted = self
            .load_internal::<u32>(RUNTIME_STATE_SCHEMA_KEY)?
            .unwrap_or(0);
        ensure!(
            persisted <= RUNTIME_STATE_SCHEMA_VERSION,
            "State database schema {} is newer than runtime schema {}",
            persisted,
            RUNTIME_STATE_SCHEMA_VERSION
        );
        if persisted < RUNTIME_STATE_SCHEMA_VERSION {
            self.save_internal(RUNTIME_STATE_SCHEMA_KEY, &RUNTIME_STATE_SCHEMA_VERSION)?;
            self.commit()?;
        }
        Ok(())
    }

    fn ensure_smt_initialized(&self) -> Result<()> {
        let Some(smt) = &self.smt else {
            return Ok(());
        };
        if smt.root_hash()? != smt::default_hashes()[0] {
            return Ok(());
        }

        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = self
            .store
            .logical_entries()
            .context("Failed to read state entries for SMT rebuild")?
            .into_iter()
            .collect();
        Self::retain_canonical_state_root_entries(&mut entries);
        if entries.is_empty() {
            return Ok(());
        }

        let updates = entries.into_iter().collect::<Vec<_>>();
        smt.insert(&updates)?;
        Ok(())
    }

    fn canonical_persisted_entries(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = self
            .store
            .logical_entries()
            .context("Failed to read canonical state entries")?
            .into_iter()
            .collect();
        Self::retain_canonical_state_root_entries(&mut entries);
        Ok(entries.into_iter().collect())
    }

    fn ensure_smt_consistent(&self) -> Result<()> {
        ensure!(
            self.overlay.is_empty(),
            "Cannot verify SMT with a pending overlay"
        );
        self.repair_persisted_smt().map(|_| ())
    }

    /// Reconcile the persisted SMT base after a component writes canonical
    /// Move modules directly to the shared store.
    pub fn repair_persisted_smt(&self) -> Result<bool> {
        let Some(smt) = &self.smt else {
            return Ok(false);
        };
        let entries = self.canonical_persisted_entries()?;
        let expected = smt::compute_sparse_root(&entries);
        if smt.root_hash()? != expected {
            log::warn!("Persisted SMT is stale; rebuilding canonical state tree");
            smt.rebuild(&entries)?;
            ensure!(
                smt.root_hash()? == expected,
                "SMT rebuild produced wrong root"
            );
            return Ok(true);
        }
        Ok(false)
    }

    /// Audit the persisted incremental tree against canonical state. This is a
    /// maintenance/diagnostic operation and intentionally performs a full scan.
    pub fn validate_smt_consistency(&self) -> Result<()> {
        let Some(smt) = &self.smt else {
            return Ok(());
        };
        ensure!(
            self.overlay.is_empty(),
            "Cannot audit SMT with a pending overlay"
        );
        let entries = self.canonical_persisted_entries()?;
        let mut mismatches = Vec::new();
        for (key, expected) in &entries {
            match smt.get(key)? {
                Some(actual) if actual == *expected => {}
                Some(actual) => mismatches.push(format!(
                    "{}(expected_bytes={}, actual_bytes={})",
                    String::from_utf8_lossy(key),
                    expected.len(),
                    actual.len()
                )),
                None => mismatches.push(format!("{}(missing)", String::from_utf8_lossy(key))),
            }
            if mismatches.len() == 8 {
                break;
            }
        }
        ensure!(
            mismatches.is_empty(),
            "SMT leaf mismatch for canonical keys: {}",
            mismatches.join(", ")
        );
        let expected_root = smt::compute_sparse_root(&entries);
        let stale_known_keys = if smt.persisted_leaf_count()? > entries.len() {
            let canonical_keys = entries
                .iter()
                .map(|(key, _)| key.as_slice())
                .collect::<HashSet<_>>();
            self.store
                .logical_entries()?
                .into_iter()
                .filter(|(key, _)| !canonical_keys.contains(key.as_slice()))
                .filter_map(|(key, _)| {
                    smt.get(&key).ok().flatten().map(|_| {
                        let label = String::from_utf8_lossy(&key).into_owned();
                        self.store
                            .load::<StoredObject>(&key)
                            .ok()
                            .flatten()
                            .map(|object| {
                                format!(
                                    "{}[owner={},kind={:?},type={}]",
                                    label,
                                    object.owner.to_hex_literal(),
                                    object.owner_kind,
                                    object.type_name
                                )
                            })
                            .unwrap_or(label)
                    })
                })
                .take(8)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        ensure!(
            smt.root_hash()? == expected_root,
            "SMT root mismatch after canonical leaf audit: canonical_leaves={} persisted_leaves={} stale_known_keys={}",
            entries.len(),
            smt.persisted_leaf_count()?,
            stale_known_keys.join(",")
        );
        Ok(())
    }

    /// Return read-only SMT status. When `audit` is true this also performs the
    /// expensive full canonical leaf/root comparison; it never repairs state.
    pub fn smt_diagnostics(&self, audit: bool) -> Result<SmtDiagnostics> {
        let (updates, deletes) = self.smt_changes_from_overlay();
        let enabled = self.smt.is_some();
        let persisted_root = self
            .smt
            .as_ref()
            .map(|tree| tree.root_hash().map(hex::encode))
            .transpose()?;

        let mut persisted_leaf_count = None;
        let mut consistent = None;
        let mut consistency_error = None;
        let audit_performed = audit && enabled;

        if audit_performed {
            persisted_leaf_count = self
                .smt
                .as_ref()
                .map(|tree| tree.persisted_leaf_count())
                .transpose()?;
            match self.validate_smt_consistency() {
                Ok(()) => consistent = Some(true),
                Err(error) => {
                    consistent = Some(false);
                    consistency_error = Some(error.to_string());
                }
            }
        }

        Ok(SmtDiagnostics {
            enabled,
            persisted_root,
            effective_root: hex::encode(self.compute_state_root()),
            overlay_entries: self.overlay.len(),
            overlay_updates: updates.len(),
            overlay_deletes: deletes.len(),
            canonical_membership_changed: self.canonical_membership_changed(),
            runtime_schema_version: self.load_internal(RUNTIME_STATE_SCHEMA_KEY)?,
            expected_runtime_schema_version: RUNTIME_STATE_SCHEMA_VERSION,
            wallet_supply_index_version: self.load_internal(WALLET_SUPPLY_INDEX_VERSION_KEY)?,
            expected_wallet_supply_index_version: WALLET_SUPPLY_INDEX_VERSION,
            audit_requested: audit,
            audit_performed,
            persisted_leaf_count,
            consistent,
            consistency_error,
        })
    }

    fn repair_derived_indexes_on_startup(&mut self) -> Result<bool> {
        let (expected_owner_ids, expected_owned_objects, expected_object_ids) =
            self.collect_derived_indexes()?;
        let mut changed = false;

        let current_owner_ids = self.load_index_list(OWNER_INDEX_KEY)?;
        if current_owner_ids != expected_owner_ids {
            self.save_index_list(OWNER_INDEX_KEY, &expected_owner_ids)?;
            changed = true;
        }

        let legacy_owner_ids = self.load_index_list(LEGACY_ACCOUNT_INDEX_KEY)?;
        if !legacy_owner_ids.is_empty() {
            self.overlay.insert(LEGACY_ACCOUNT_INDEX_KEY.to_vec(), None);
            changed = true;
        }

        let current_object_ids = self.load_index_list(b"object_index")?;
        if current_object_ids != expected_object_ids {
            self.save_index_list(b"object_index", &expected_object_ids)?;
            changed = true;
        }

        let indexed_owners = current_owner_ids
            .into_iter()
            .chain(expected_owner_ids.iter().cloned())
            .filter_map(|id| AccountAddress::from_hex_literal(&id).ok())
            .collect::<BTreeSet<_>>();

        for owner in indexed_owners {
            let expected_ids = expected_owned_objects
                .get(&owner)
                .cloned()
                .unwrap_or_default();
            let key = owned_objects_key(&owner);
            let current_ids = self.load_index_list(&key)?;
            if current_ids != expected_ids {
                self.save_index_list(&key, &expected_ids)?;
                changed = true;
            }
        }

        Ok(changed)
    }

    /// Commit pending overlay changes to the persistent store and update SMT
    pub fn commit(&mut self) -> Result<()> {
        let mut updates = Vec::new();
        let mut deletes = Vec::new();

        for (key, val_opt) in &self.overlay {
            if let Some(val) = val_opt {
                updates.push((key.clone(), val.clone()));
            } else {
                deletes.push(key.clone());
            }
        }

        let canonical_membership_changed = self.canonical_membership_changed();
        // Derive SMT deltas before writing the overlay so index transitions can
        // compare the old persisted membership with the new overlay membership.
        let smt_changes = self.smt.as_ref().map(|_| self.smt_changes_from_overlay());

        self.store.apply_raw_changes(&updates, &deletes)?;

        // Update SMT if available
        if let (Some(smt), Some((smt_updates, smt_deletes))) = (&self.smt, smt_changes) {
            let update_result = (|| -> Result<()> {
                if !smt_deletes.is_empty() {
                    smt.delete(&smt_deletes)?;
                }
                if !smt_updates.is_empty() {
                    smt.insert(&smt_updates)?;
                }
                Ok(())
            })();
            if let Err(update_error) = update_result {
                // Canonical state has already committed. Repair immediately so callers
                // never receive a simple failure while the live DB remains split-brain.
                self.repair_persisted_smt().with_context(|| {
                    format!("SMT delta update failed ({update_error}); full repair also failed")
                })?;
            }
        }

        if canonical_membership_changed {
            self.repair_persisted_smt()?;
        }

        self.overlay.clear();
        Ok(())
    }

    // Helper to write to overlay (pub for genesis module)
    pub(crate) fn save_internal<T: Serialize + ?Sized>(
        &mut self,
        key: &[u8],
        value: &T,
    ) -> Result<()> {
        let bytes = bcs::to_bytes(value)?;
        self.overlay.insert(key.to_vec(), Some(bytes));
        Ok(())
    }

    // Helper to read from overlay then store
    pub fn load_internal<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>> {
        if let Some(val_opt) = self.overlay.get(key) {
            match val_opt {
                Some(bytes) => return Ok(Some(bcs::from_bytes(bytes)?)),
                None => return Ok(None),
            }
        }
        Ok(self.store.load(key)?)
    }

    // Helper to construct DB key for one owner-state record.
    fn owner_state_key(address: &AccountAddress) -> Vec<u8> {
        let mut key = b"account:".to_vec();
        key.extend_from_slice(address.as_ref());
        key
    }

    // Helper to construct DB key for object

    fn object_lookup_ids(id: &str) -> Vec<String> {
        let mut ids = vec![id.to_string()];
        if let Some(canonical_id) = canonical_object_id(id)
            && canonical_id != id
        {
            ids.push(canonical_id);
        }
        ids
    }

    fn load_stored_object_by_any_id(
        &self,
        object_id: &str,
    ) -> Result<Option<(String, StoredObject)>> {
        for candidate_id in Self::object_lookup_ids(object_id) {
            let obj_key = object_key(&candidate_id);
            if let Some(stored) = self.load_internal::<StoredObject>(&obj_key)? {
                return Ok(Some((candidate_id, stored)));
            }
        }
        Ok(None)
    }

    // helper for generating DB keys for Dynamic Fields
    fn dynamic_field_key(object_id: &str, name_bytes: &[u8]) -> Vec<u8> {
        let hash = hash_data_blake3(name_bytes);
        let mut key = b"df:".to_vec();
        key.extend_from_slice(object_id.as_bytes());
        key.extend_from_slice(b":");
        key.extend_from_slice(hex::encode(&hash[0..16]).as_bytes());
        key
    }

    fn is_canonical_state_root_key(key: &[u8]) -> bool {
        key != GENESIS_INITIALIZED_KEY
            && (key == b"total_supply"
                || key.starts_with(b"resource:")
                || key.starts_with(b"account:")
                || key.starts_with(b"df:")
                || key.starts_with(b"system:")
                || key.starts_with(b"supply:")
                || key.starts_with(b"treasury:")
                || key.starts_with(b"nft:"))
    }

    fn retain_canonical_state_root_entries(entries: &mut BTreeMap<Vec<u8>, Vec<u8>>) {
        let mut canonical_object_keys = BTreeSet::new();
        let mut canonical_module_keys = BTreeSet::new();

        for (key, value) in entries.iter() {
            if key == b"module_index" {
                if let Ok(module_ids) = bcs::from_bytes::<Vec<String>>(value) {
                    canonical_module_keys.extend(
                        module_ids
                            .into_iter()
                            .filter(|id| id.starts_with("module:"))
                            .map(String::into_bytes),
                    );
                }
            } else if key.starts_with(b"owned_objects:")
                && let Ok(object_ids) = bcs::from_bytes::<Vec<String>>(value)
            {
                canonical_object_keys.extend(
                    object_ids
                        .into_iter()
                        .filter_map(|id| canonical_object_id(&id))
                        .filter(|id| {
                            entries
                                .get(&object_key(id))
                                .and_then(|bytes| bcs::from_bytes::<StoredObject>(bytes).ok())
                                .is_some()
                        })
                        .map(|id| object_key(&id)),
                );
            }
        }

        entries.retain(|key, _| {
            Self::is_canonical_state_root_key(key)
                || canonical_module_keys.contains(key)
                || canonical_object_keys.contains(key)
        });
    }

    fn is_canonical_smt_update(&self, key: &[u8], value: &[u8]) -> bool {
        if Self::is_canonical_state_root_key(key) {
            return true;
        }
        if let Some(module_id) = key.strip_prefix(b"module:")
            && let Ok(module_id) = std::str::from_utf8(module_id)
        {
            return self
                .load_index_list(b"module_index")
                .is_ok_and(|ids| ids.iter().any(|id| id == &format!("module:{module_id}")));
        }
        if let Some(object_id) = key.strip_prefix(b"object:")
            && let Ok(object_id) = std::str::from_utf8(object_id)
            && let Ok(object) = bcs::from_bytes::<StoredObject>(value)
            && matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
        {
            let canonical_id = canonical_object_id(object_id).unwrap_or_else(|| object_id.into());
            return self
                .load_index_list(&owned_objects_key(&object.owner))
                .is_ok_and(|ids| ids.binary_search(&canonical_id).is_ok());
        }
        false
    }

    fn smt_changes_from_overlay(&self) -> OverlaySmtChanges {
        let mut updates = Vec::new();
        let mut deletes = Vec::new();

        for (key, value_opt) in &self.overlay {
            match value_opt {
                Some(value) if self.is_canonical_smt_update(key, value) => {
                    updates.push((key.clone(), value.clone()));
                }
                // An object/module may transition out of the canonical root
                // set (for example AddressOwner -> Shared). A non-canonical
                // replacement must remove the previously indexed leaf.
                Some(_) if key.starts_with(b"module:") || key.starts_with(b"object:") => {
                    deletes.push(key.clone());
                }
                None if Self::is_canonical_state_root_key(key)
                    || key.starts_with(b"module:")
                    || key.starts_with(b"object:") =>
                {
                    deletes.push(key.clone());
                }
                _ => {}
            }
        }

        // Object membership in the canonical root is driven by owned_objects
        // indexes. An index can drop/add an unchanged object without placing
        // that object's bytes in the overlay, so derive those leaf deltas too.
        for (index_key, value_opt) in self
            .overlay
            .iter()
            .filter(|(key, _)| key.starts_with(b"owned_objects:"))
        {
            let old_ids = self
                .store
                .load::<Vec<String>>(index_key)
                .ok()
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| canonical_object_id(&id))
                .collect::<BTreeSet<_>>();
            let new_ids = value_opt
                .as_ref()
                .and_then(|bytes| bcs::from_bytes::<Vec<String>>(bytes).ok())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| canonical_object_id(&id))
                .collect::<BTreeSet<_>>();

            for removed in old_ids.difference(&new_ids) {
                deletes.push(object_key(removed));
            }
            for added in new_ids.difference(&old_ids) {
                let key = object_key(added);
                if let Ok(Some(object)) = self.load_internal::<StoredObject>(&key)
                    && matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
                    && let Ok(value) = bcs::to_bytes(&object)
                {
                    updates.push((key, value));
                }
            }
        }

        updates.sort_by(|left, right| left.0.cmp(&right.0));
        updates.dedup_by(|left, right| left.0 == right.0);
        let updated_keys = updates
            .iter()
            .map(|(key, _)| key.as_slice())
            .collect::<HashSet<_>>();
        deletes.retain(|key| !updated_keys.contains(key.as_slice()));
        deletes.sort();
        deletes.dedup();

        (updates, deletes)
    }

    fn canonical_membership_changed(&self) -> bool {
        self.overlay.iter().any(|(key, value_opt)| {
            if key != b"module_index" && !key.starts_with(b"owned_objects:") {
                return false;
            }
            let old = self.store.load::<Vec<String>>(key).ok().flatten();
            let new = value_opt
                .as_ref()
                .and_then(|bytes| bcs::from_bytes::<Vec<String>>(bytes).ok());
            old != new
        })
    }

    pub(crate) fn get_system_clock_object_id(&self) -> Result<Option<AccountAddress>> {
        let bytes_opt: Option<Vec<u8>> = self.load_internal(SYSTEM_CLOCK_OBJECT_ID_KEY)?;
        match bytes_opt {
            None => Ok(None),
            Some(bytes) => {
                let addr = AccountAddress::from_bytes(bytes)?;
                Ok(Some(addr))
            }
        }
    }

    pub(crate) fn set_system_clock_object_id(&mut self, id: AccountAddress) -> Result<()> {
        self.save_internal(SYSTEM_CLOCK_OBJECT_ID_KEY, &id.as_ref().to_vec())
    }

    pub fn apply_zero_effect_sequence_batch<I>(&mut self, sequence_increments: I) -> Result<()>
    where
        I: IntoIterator<Item = (AccountAddress, u64)>,
    {
        let _ = sequence_increments;
        Ok(())
    }

    fn save_owner_record(&mut self, owner_state: &OwnerState) -> Result<()> {
        self.save_internal(&Self::owner_state_key(&owner_state.address), owner_state)
    }

    fn add_many_to_index_list<I>(&mut self, key: &[u8], values: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        let existing = self.load_index_list(key)?;
        let mut index: BTreeSet<String> = existing.into_iter().collect();
        let mut changed = false;

        for value in values {
            changed |= index.insert(value);
        }

        if changed {
            let sorted = index.into_iter().collect::<Vec<_>>();
            self.save_index_list(key, &sorted)?;
        }

        Ok(())
    }

    pub fn owner_addresses(&self) -> Result<Vec<AccountAddress>> {
        let ids = self.load_owner_index_ids()?;
        Ok(ids
            .into_iter()
            .filter_map(|id| AccountAddress::from_hex_literal(&id).ok())
            .collect())
    }

    fn load_owner_index_ids(&self) -> Result<Vec<String>> {
        let owner_ids = self.load_index_list(OWNER_INDEX_KEY)?;
        if !owner_ids.is_empty() {
            return Ok(owner_ids);
        }

        self.load_index_list(LEGACY_ACCOUNT_INDEX_KEY)
    }

    fn load_owner_state_or_default(&self, address: AccountAddress) -> Result<OwnerState> {
        Ok(self
            .load_owner_state(&address)?
            .unwrap_or_else(|| OwnerState::new(address)))
    }

    pub fn load_owner_state(&self, owner: &AccountAddress) -> Result<Option<OwnerState>> {
        self.load_internal(&Self::owner_state_key(owner))
    }

    fn save_owner_state_without_supply_index(&mut self, owner_state: &OwnerState) -> Result<()> {
        if owner_state.is_empty() {
            self.overlay
                .insert(Self::owner_state_key(&owner_state.address), None);
            self.remove_from_index_list(OWNER_INDEX_KEY, &owner_state.address.to_hex_literal())
        } else {
            self.save_owner_record(owner_state)?;
            self.add_to_index_list(OWNER_INDEX_KEY, owner_state.address.to_hex_literal())
        }
    }

    pub fn save_owner_state(&mut self, owner_state: &OwnerState) -> Result<()> {
        let old_balances = self
            .load_owner_state(&owner_state.address)?
            .map(|state| state.token_balances)
            .unwrap_or_default();
        self.save_owner_state_without_supply_index(owner_state)?;
        let old_native = old_balances
            .get(GAS_COIN)
            .map(|balance| balance.value())
            .unwrap_or(0);
        let new_native = owner_state.native_balance();
        let cached_native = self
            .global_token_supplies
            .get(GAS_COIN)
            .copied()
            .unwrap_or(0);
        let projected_native =
            u128::from(cached_native.saturating_sub(old_native)) + u128::from(new_native);
        let update_supply_index =
            old_native == new_native || projected_native <= u128::from(self.total_supply);
        if update_supply_index && self.capture_supply_changed(owner_state, &old_balances)? {
            let supplies = self.global_token_supplies.clone();
            self.save_internal(b"global_token_supplies", &supplies)?;
        }
        Ok(())
    }

    pub fn get_owner_state(&self, owner: &AccountAddress) -> Option<OwnerState> {
        match self.load_owner_state(owner) {
            Ok(account) => account,
            Err(e) => {
                log::error!(
                    "[StateManager] Failed to load owner state {}: {}",
                    owner.to_hex_literal(),
                    e
                );
                None
            }
        }
    }

    pub fn get_owner_state_by_hex(&self, owner: &str) -> Option<OwnerState> {
        // Use Address::parse_to_account_address which handles tagged addresses,
        // tagged public keys (hashing), and regular 0x addresses.
        if let Ok(addr) = kanari_types::address::Address::parse_to_account_address(owner) {
            self.get_owner_state(&addr)
        } else {
            log::warn!(
                "[StateManager] Failed to parse owner address from hex: {}",
                owner
            );
            None
        }
    }

    fn balance_token_amount(type_name: &str, data: &[u8]) -> Option<(String, u64)> {
        let struct_tag = StructTag::from_str(type_name).ok()?;
        if !Self::is_balance_struct(&struct_tag) {
            return None;
        }
        let token_type = Self::token_type_from_balance_struct(&struct_tag)?;
        let amount = Self::extract_balance_from_object_bytes(data, &struct_tag)?;
        Some((Self::normalize_token_type(&token_type), amount))
    }

    /// Get all object IDs owned by an address
    pub fn get_owned_objects(&self, owner: &AccountAddress) -> Result<Vec<String>> {
        let owner_key = owned_objects_key(owner);
        let mut clean: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
        clean.sort();
        clean.dedup();
        Ok(clean)
    }

    /// Get a specific object by ID
    pub fn get_object(&self, object_id: &str) -> Result<Option<CreatedObject>> {
        let Some((stored_id, stored)) = self.load_stored_object_by_any_id(object_id)? else {
            return Ok(None);
        };
        let normalized_id = canonical_object_id(&stored_id).unwrap_or(stored_id.clone());
        let uid = AccountAddress::from_hex_literal(&normalized_id)
            .ok()
            .map(UIDRecord::new);
        let id = AccountAddress::from_hex_literal(&normalized_id)
            .ok()
            .map(IDRecord::new);

        Ok(Some(CreatedObject {
            owner: stored.owner,
            owner_kind: stored.owner_kind,
            uid,
            id,
            type_: stored.type_name,
            data: stored.data,
            version: stored.version,
        }))
    }

    pub fn get_objects_by_type(&self, object_type: &str) -> Result<Vec<(String, CreatedObject)>> {
        self.query_objects(None, None, Some(object_type), None, None)
    }

    pub fn query_objects(
        &self,
        owner: Option<AccountAddress>,
        owner_kind: Option<&ObjectOwnerKind>,
        object_type: Option<&str>,
        min_version: Option<u64>,
        max_version: Option<u64>,
    ) -> Result<Vec<(String, CreatedObject)>> {
        let object_ids: Vec<String> = self.load_internal(b"object_index")?.unwrap_or_default();
        let mut objects = Vec::new();

        for object_id in object_ids {
            if let Some(object) = self.get_object(&object_id)? {
                if let Some(owner) = owner
                    && object.owner != owner
                {
                    continue;
                }
                if let Some(owner_kind) = owner_kind {
                    let matches = match owner_kind {
                        ObjectOwnerKind::AddressOwner(_) => {
                            matches!(object.owner_kind, ObjectOwnerKind::AddressOwner(_))
                        }
                        ObjectOwnerKind::Shared => {
                            matches!(object.owner_kind, ObjectOwnerKind::Shared)
                        }
                        ObjectOwnerKind::Immutable => {
                            matches!(object.owner_kind, ObjectOwnerKind::Immutable)
                        }
                    };
                    if !matches {
                        continue;
                    }
                }
                if let Some(object_type) = object_type
                    && object.type_ != object_type
                {
                    continue;
                }
                if let Some(min_version) = min_version
                    && object.version < min_version
                {
                    continue;
                }
                if let Some(max_version) = max_version
                    && object.version > max_version
                {
                    continue;
                }

                objects.push((object_id, object));
            }
        }

        Ok(objects)
    }

    pub fn compute_state_root(&self) -> Vec<u8> {
        if let Some(smt) = &self.smt
            && !self.canonical_membership_changed()
        {
            let (updates, deletes) = self.smt_changes_from_overlay();
            match smt.root_hash_with_changes(&updates, &deletes) {
                Ok(root) => return root.to_vec(),
                Err(error) => {
                    log::error!("Failed to compute incremental state root: {}", error);
                }
            }
        }
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = match self.store.logical_entries() {
            Ok(entries) => entries.into_iter().collect(),
            Err(e) => {
                log::error!("Failed to materialize state root snapshot: {}", e);
                BTreeMap::new()
            }
        };

        for (key, value_opt) in &self.overlay {
            if let Some(value) = value_opt {
                entries.insert(key.clone(), value.clone());
            } else {
                entries.remove(key);
            }
        }

        Self::retain_canonical_state_root_entries(&mut entries);

        smt::compute_sparse_root(&entries.into_iter().collect::<Vec<_>>()).to_vec()
    }

    pub fn canonical_state_snapshot(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut entries: BTreeMap<Vec<u8>, Vec<u8>> = match self.store.logical_entries() {
            Ok(entries) => entries.into_iter().collect(),
            Err(e) => {
                log::error!("Failed to materialize canonical state snapshot: {}", e);
                BTreeMap::new()
            }
        };

        for (key, value_opt) in &self.overlay {
            if let Some(value) = value_opt {
                entries.insert(key.clone(), value.clone());
            } else {
                entries.remove(key);
            }
        }

        Self::retain_canonical_state_root_entries(&mut entries);
        entries
    }

    pub fn resolve_owner_nonce(&self, _owner: &AccountAddress) -> Result<u64> {
        Ok(0)
    }

    /// Legacy no-op: owner/account nonce is not an execution validity rule.
    pub fn validate_owner_nonce(
        &self,
        _owner: &AccountAddress,
        _expected_nonce: u64,
    ) -> Result<()> {
        Ok(())
    }

    /// Get the total number of owners with persisted owner state.
    pub fn owner_count(&self) -> usize {
        self.load_owner_index_ids()
            .map(|owner_ids| owner_ids.len())
            .unwrap_or(0)
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
