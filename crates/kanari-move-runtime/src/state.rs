// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::changeset::{ChangeSet, CreatedObject};
use crate::storage::persistent_store::PersistentStore;
use anyhow::Result;
use kanari_types::balance::BalanceRecord;
use kanari_types::coin::TreasuryCap;
use kanari_types::kanari::KanariModule;
use kanari_types::{address::Address as KanariAddress, event::Event};
// use log::{debug, info};
use move_core_types::account_address::AccountAddress;
// use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smt;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Account state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: AccountAddress,
    pub balance: u64, // Native KANARI balance in Mist
    pub sequence_number: u64,
    pub modules: BTreeSet<String>,
    /// Token balances: token_type -> BalanceRecord
    /// Example: "0x840512ff...::james::JAMES" -> BalanceRecord(1000000000)
    pub token_balances: BTreeMap<String, BalanceRecord>,
}

impl Account {
    pub fn new(address: AccountAddress, balance: u64) -> Self {
        Self {
            address,
            balance,
            sequence_number: 0,
            modules: BTreeSet::new(),
            token_balances: BTreeMap::new(),
        }
    }

    pub fn add_module(&mut self, module_name: String) {
        self.modules.insert(module_name);
    }

    pub fn set_token_balance(&mut self, token_type: String, amount: BalanceRecord) {
        self.token_balances.insert(token_type, amount);
    }

    pub fn get_token_balance(&self, token_type: &str) -> u64 {
        self.token_balances
            .get(token_type)
            .map(|b| b.value())
            .unwrap_or(0)
    }

    pub fn add_token(&mut self, token_type: String, amount: u64) {
        let entry = self
            .token_balances
            .entry(token_type)
            .or_insert_with(BalanceRecord::zero);
        entry.increase(amount).unwrap();
    }

    pub fn sub_token(&mut self, token_type: &str, amount: u64) -> Result<()> {
        if let Some(bal) = self.token_balances.get_mut(token_type) {
            bal.decrease(amount)?;
            Ok(())
        } else {
            anyhow::bail!("Insufficient token balance");
        }
    }

    pub fn to_hex_string(&self) -> String {
        format!("{:#x}", self.address)
    }

    pub fn increment_sequence(&mut self) {
        self.sequence_number += 1;
    }
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

    // SMT for state root calculation (Optional: requires DB backend)
    pub smt: Option<Arc<smt::SparseMerkleTree>>,

    // Events are accumulated in memory per block and flushed elsewhere
    pub events: Vec<Event>,
}

impl StateManager {
    /// Create a new in-memory state manager for testing
    pub fn new_in_memory() -> Self {
        let store =
            Arc::new(PersistentStore::open_in_memory().expect("Failed to create in-memory store"));
        Self::new(store)
    }

    /// Create new state with genesis allocation
    /// Total supply: 11 million KANARI = 11,000,000,000,000,000 Mist
    /// Dev address gets entire supply according to kanari.move
    pub fn new(store: Arc<PersistentStore>) -> Self {
        // Try to load total supply from DB
        let total_supply = store
            .load::<u64>(b"total_supply")
            .unwrap_or(None)
            .unwrap_or(0);

        // Initialize SMT if store is backed by RocksDB
        let smt = store
            .get_db()
            .map(|db| Arc::new(smt::SparseMerkleTree::new(db)));

        let mut state = Self {
            store,
            overlay: BTreeMap::new(),
            total_supply,
            smt,
            events: Vec::new(),
        };

        // If total supply is 0, initialize genesis
        if state.total_supply == 0 {
            let _ = state.init_genesis();
            // Flush genesis state to DB immediately
            let _ = state.commit();
        }

        state
    }

    fn init_genesis(&mut self) -> Result<()> {
        // Total supply in Mist (from kanari-types constants)
        let total_supply_mist: u64 = KanariModule::TOTAL_SUPPLY_MIST;

        // Initialize system accounts using safe helper functions
        let genesis_addr = KanariAddress::genesis_account_address();
        let std_addr = KanariAddress::std_account_address();
        let system_addr = KanariAddress::kanari_system_account_address();
        let dao_addr = KanariAddress::dao_account_address();
        let dev_addr = KanariAddress::dev_account_address();

        self.save_account(&Account::new(genesis_addr, 0))?;
        self.save_account(&Account::new(std_addr, 0))?;
        self.save_account(&Account::new(system_addr, 0))?;
        self.save_account(&Account::new(dao_addr, 0))?;
        self.save_account(&Account::new(dev_addr, total_supply_mist))?;

        self.total_supply = total_supply_mist;
        let supply = self.total_supply;
        self.save_internal(b"total_supply", &supply)?;

        Ok(())
    }

    /// Commit pending overlay changes to the persistent store and update SMT
    pub fn commit(&mut self) -> Result<()> {
        let mut updates = Vec::new();
        let mut deletes = Vec::new();
        let mut batch = rocksdb::WriteBatch::default();

        for (key, val_opt) in &self.overlay {
            if let Some(val) = val_opt {
                batch.put(key, val);
                updates.push((key.clone(), val.clone()));
            } else {
                batch.delete(key);
                deletes.push(key.clone());
            }
        }

        // Apply batch to RocksDB atomically
        self.store.apply_batch(batch)?;

        // Update SMT if available
        if let Some(smt) = &self.smt {
            if !updates.is_empty() {
                smt.insert(&updates)?;
            }
            if !deletes.is_empty() {
                smt.delete(&deletes)?;
            }
        }

        self.overlay.clear();
        Ok(())
    }

    /// Load persisted treasuries as a vector of tuples (owner, token_type, TreasuryCap)
    pub fn load_treasuries(&self) -> Result<Vec<(AccountAddress, String, TreasuryCap)>> {
        let mut out = Vec::new();
        // The treasury index key used in MoveVMState
        if let Ok(Some(keys)) = self.store.load::<Vec<String>>(b"treasury_index") {
            for key in keys.into_iter() {
                // key format: "treasury:<token_type>"
                // value format: (owner_addr, TreasuryCap)
                if let Ok(Some((owner_addr, cap))) = self
                    .store
                    .load::<(AccountAddress, TreasuryCap)>(key.as_bytes())
                {
                    let token_type = key.strip_prefix("treasury:").unwrap_or(&key).to_string();
                    out.push((owner_addr, token_type, cap));
                }
            }
        }
        Ok(out)
    }

    /// Discard pending overlay changes
    pub fn discard(&mut self) {
        self.overlay.clear();
    }

    // Helper to write to overlay
    fn save_internal<T: Serialize + ?Sized>(&mut self, key: &[u8], value: &T) -> Result<()> {
        let bytes = bcs::to_bytes(value)?;
        self.overlay.insert(key.to_vec(), Some(bytes));
        Ok(())
    }

    // Helper to read from overlay then store
    fn load_internal<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>> {
        if let Some(val_opt) = self.overlay.get(key) {
            match val_opt {
                Some(bytes) => return Ok(Some(bcs::from_bytes(bytes)?)),
                None => return Ok(None),
            }
        }
        Ok(self.store.load(key)?)
    }

    // Helper to construct DB key for account
    fn account_key(address: &AccountAddress) -> Vec<u8> {
        let mut key = b"account:".to_vec();
        key.extend_from_slice(address.as_ref());
        key
    }

    // Helper to construct DB key for object
    fn object_key(id: &str) -> Vec<u8> {
        let mut key = b"object:".to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }

    // Helper to construct DB key for owned objects
    fn owned_objects_key(address: &AccountAddress) -> Vec<u8> {
        let mut key = b"owned:".to_vec();
        key.extend_from_slice(address.as_ref());
        key
    }

    pub fn load_account(&self, address: &AccountAddress) -> Result<Option<Account>> {
        self.load_internal(&Self::account_key(address))
    }

    pub fn save_account(&mut self, account: &Account) -> Result<()> {
        self.save_internal(&Self::account_key(&account.address), account)
    }

    fn load_account_or_default(&self, address: AccountAddress) -> Result<Account> {
        Ok(self
            .load_account(&address)?
            .unwrap_or_else(|| Account::new(address, 0)))
    }

    pub fn get_account(&self, address: &AccountAddress) -> Option<Account> {
        self.load_account(address).ok().flatten()
    }

    pub fn get_account_by_hex(&self, hex_address: &str) -> Option<Account> {
        // Use Address::parse_to_account_address which handles tagged addresses,
        // tagged public keys (hashing), and regular 0x addresses.
        if let Ok(addr) = kanari_types::address::Address::parse_to_account_address(hex_address) {
            self.get_account(&addr)
        } else {
            None
        }
    }

    /// Apply ChangeSet from Move VM execution
    /// This is the ONLY way to modify state - all changes must come from Move VM
    pub fn apply_changeset(&mut self, changeset: &ChangeSet) -> Result<()> {
        // Track total supply change (for mint/burn operations)
        let mut supply_delta: i64 = 0;

        for (address, change) in &changeset.account_changes {
            let mut account = self.load_account_or_default(*address)?;

            // Apply balance delta
            if change.balance_delta > 0 {
                let amount = change.balance_delta as u64;
                account.balance = account
                    .balance
                    .checked_add(amount)
                    .ok_or_else(|| anyhow::anyhow!("Balance overflow"))?;
                supply_delta += change.balance_delta;
            } else if change.balance_delta < 0 {
                let debit = (-change.balance_delta) as u64;
                if account.balance < debit {
                    anyhow::bail!(
                        "Insufficient balance for address {:#x}: need {} but have {}",
                        address,
                        debit,
                        account.balance
                    );
                }
                account.balance -= debit;
                supply_delta += change.balance_delta;
            }

            // Apply sequence number increment
            account.sequence_number += change.sequence_increment;

            // Apply module additions
            for module_name in &change.modules_added {
                account.add_module(module_name.clone());
            }

            self.save_account(&account)?;
        }

        // Update total supply if there was mint/burn (supply_delta != 0)
        if supply_delta != 0 {
            if supply_delta > 0 {
                self.total_supply = self
                    .total_supply
                    .checked_add(supply_delta as u64)
                    .ok_or_else(|| anyhow::anyhow!("Total supply overflow"))?;
            } else {
                let burn_amount = (-supply_delta) as u64;
                if self.total_supply < burn_amount {
                    anyhow::bail!("Cannot burn more than total supply");
                }
                self.total_supply -= burn_amount;
            }
            let supply = self.total_supply;
            self.save_internal(b"total_supply", &supply)?;
        }

        // Apply treasury creations/updates
        for (owner, token_type, total_supply) in &changeset.treasuries {
            let mut key = b"supply:".to_vec();
            key.extend_from_slice(token_type.as_bytes());
            self.save_internal(&key, total_supply)?;

            let mut key_owner = b"treasury:".to_vec();
            key_owner.extend_from_slice(token_type.as_bytes());
            self.save_internal(&key_owner, owner)?;

            // Update treasury index
            let mut index: Vec<String> = self.load_internal(b"treasury_index")?.unwrap_or_default();
            let key_str = format!("treasury:{}", token_type);
            if !index.contains(&key_str) {
                index.push(key_str);
                self.save_internal(b"treasury_index", &index)?;
            }
        }

        // Apply NFT capability creations/updates
        for (owner, token_type, nft_cap) in &changeset.nft_caps {
            let mut key = b"nft:".to_vec();
            key.extend_from_slice(token_type.as_bytes());
            self.save_internal(&key, &(*owner, nft_cap.clone()))?;
        }

        // Apply per-account token balance sets
        for (owner, token_type, amount) in &changeset.token_balance_sets {
            let mut account = self.load_account_or_default(*owner)?;
            account.set_token_balance(token_type.clone(), amount.clone());
            self.save_account(&account)?;
        }

        // Persist created objects
        for (obj_id, created) in &changeset.created_objects {
            let obj_key = Self::object_key(obj_id);

            // Check if object exists and handle ownership transfer
            // This is crucial: if an object is transferred, we must remove it from the old owner's list
            if let Ok(Some(existing)) = self.load_internal::<CreatedObject>(&obj_key) {
                if existing.owner != created.owner {
                    // Remove from old owner's list
                    let old_owner_key = Self::owned_objects_key(&existing.owner);
                    if let Ok(Some(mut old_owned)) =
                        self.load_internal::<Vec<String>>(&old_owner_key)
                    {
                        if let Some(pos) = old_owned.iter().position(|x| x == obj_id) {
                            old_owned.remove(pos);
                            self.save_internal(&old_owner_key, &old_owned)?;
                        }
                    }
                }
            }

            // Store the object
            self.save_internal(&obj_key, created)?;

            // Update owned objects list
            let owner_key = Self::owned_objects_key(&created.owner);
            let mut owned: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
            // Avoid duplicates
            if !owned.contains(obj_id) {
                owned.push(obj_id.clone());
                self.save_internal(&owner_key, &owned)?;
            }
        }

        // Process deleted objects
        for obj_id in &changeset.deleted_objects {
            // Remove the object
            let obj_key = Self::object_key(obj_id);
            // We need to know the owner to remove it from their owned list.
            // Since we are deleting, we first load the object to get the owner.
            if let Some(obj_data) = self.load_internal::<CreatedObject>(&obj_key)? {
                // Remove from owner's list
                let owner_key = Self::owned_objects_key(&obj_data.owner);
                if let Some(mut owned) = self.load_internal::<Vec<String>>(&owner_key)? {
                    if let Some(pos) = owned.iter().position(|x| x == obj_id) {
                        owned.remove(pos);
                        self.save_internal(&owner_key, &owned)?;
                    }
                }
            }

            // Delete from storage (overlay -> DB)
            self.overlay.insert(obj_key, None);
        }

        // Persist events emitted by Move VM into state event store
        if !changeset.events.is_empty() {
            self.events.extend(changeset.events.clone());
            // Sort events by sequence number to ensure deterministic order
            self.events
                .sort_by_key(|e| (e.sequence_number, e.type_tag.to_string()));
        }

        Ok(())
    }

    /// Validate transaction sequence number before execution
    pub fn validate_sequence(
        &self,
        address: &AccountAddress,
        expected_sequence: u64,
    ) -> Result<()> {
        if let Some(account) = self.get_account(address) {
            if account.sequence_number != expected_sequence {
                anyhow::bail!(
                    "Sequence number mismatch for {:#x}: expected {}, got {}",
                    address,
                    account.sequence_number,
                    expected_sequence
                );
            }
        } else if expected_sequence != 0 {
            anyhow::bail!(
                "Account {:#x} does not exist, expected sequence must be 0",
                address
            );
        }
        Ok(())
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        // Use Address::parse_to_account_address which handles tagged addresses,
        // tagged public keys (hashing), and regular 0x addresses.
        if let Ok(addr) = kanari_types::address::Address::parse_to_account_address(address) {
            self.get_account(&addr).map(|acc| acc.balance).unwrap_or(0)
        } else {
            0
        }
    }

    // Note: account_count() is expensive with DB, so removed or needs full scan.
    // For now, removing it or returning 0.
    pub fn account_count(&self) -> usize {
        0 // Not supported efficiently in DB mode
    }

    pub fn compute_state_root(&self) -> Vec<u8> {
        self.smt
            .as_ref()
            .and_then(|smt| smt.root_hash().ok())
            .map(|root| root.to_vec())
            .unwrap_or_else(|| vec![0u8; 32])
    }

    pub fn get_object(&self, id: &str) -> Result<Option<CreatedObject>> {
        self.load_internal(&Self::object_key(id))
    }

    pub fn get_owned_objects(&self, address: &AccountAddress) -> Result<Vec<String>> {
        Ok(self
            .load_internal(&Self::owned_objects_key(address))?
            .unwrap_or_default())
    }

    pub fn get_token_supply(&self, token_type: &str) -> Result<Option<u64>> {
        let mut key = b"supply:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal(&key)
    }
}
