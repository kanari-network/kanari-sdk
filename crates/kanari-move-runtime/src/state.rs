// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::changeset::{ChangeSet, CreatedObject};
use crate::storage::persistent_store::PersistentStore;
use anyhow::Result;
use kanari_types::balance::BalanceRecord;
use kanari_types::coin::TreasuryCap;
use kanari_types::kanari::KanariModule;
use kanari_types::{address::Address as KanariAddress, event::Event};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::TypeTag;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smt;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::str::FromStr;
use std::sync::Arc;

const SYSTEM_CLOCK_OBJECT_ID_KEY: &[u8] = b"system:clock_object_id";

/// Account state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: AccountAddress,
    pub balance: u64,
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

    /// In-memory cache for tracking total token supplies in real-time
    pub global_token_supplies: BTreeMap<String, u64>,

    // SMT for state root calculation (Optional: requires DB backend)
    pub smt: Option<Arc<smt::SparseMerkleTree>>,
    pub events: Vec<Event>,
}

impl StateManager {
    /// ดึงรายการ Collection IDs ทั้งหมดจาก Index
    pub fn get_all_collection_ids(&self) -> Vec<String> {
        self.load_internal::<Vec<String>>(b"nft_collection_index")
            .unwrap_or(None)
            .unwrap_or_default()
    }

    /// ดึงรายการ NFT IDs ใน Collection นั้นๆ จาก Index
    pub fn get_collection_nft_ids(&self, collection_id: &str) -> Vec<String> {
        let mut key = b"collection_members:".to_vec();
        key.extend_from_slice(collection_id.as_bytes());

        self.load_internal::<Vec<String>>(&key)
            .unwrap_or(None)
            .unwrap_or_default()
    }

    fn normalize_token_type(token_type: &str) -> String {
        if let Ok(TypeTag::Struct(st)) = TypeTag::from_str(token_type) {
            return format!("{}", st);
        }
        token_type.to_string()
    }

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

        // Load existing global token supplies from RocksDB
        let global_token_supplies = store
            .load::<BTreeMap<String, u64>>(b"global_token_supplies")
            .unwrap_or(None)
            .unwrap_or_default();

        // Initialize SMT if store is backed by RocksDB
        let smt = store
            .get_db()
            .map(|db| Arc::new(smt::SparseMerkleTree::new(db)));

        let mut state = Self {
            store,
            overlay: BTreeMap::new(),
            total_supply,
            global_token_supplies,
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
    fn owned_objects_key(owner: &AccountAddress) -> Vec<u8> {
        let mut key = b"owned_objects:".to_vec();
        key.extend_from_slice(owner.as_ref());
        key
    }

    pub fn get_system_clock_object_id(&self) -> Result<Option<AccountAddress>> {
        let bytes_opt: Option<Vec<u8>> = self.load_internal(SYSTEM_CLOCK_OBJECT_ID_KEY)?;
        match bytes_opt {
            None => Ok(None),
            Some(bytes) => {
                let addr = AccountAddress::from_bytes(bytes)?;
                Ok(Some(addr))
            }
        }
    }

    pub fn set_system_clock_object_id(&mut self, id: AccountAddress) -> Result<()> {
        self.save_internal(SYSTEM_CLOCK_OBJECT_ID_KEY, &id.as_ref().to_vec())
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
        let mut supply_delta: i64 = 0;

        for (address, change) in &changeset.account_changes {
            let mut account = self.load_account_or_default(*address)?;

            if change.balance_delta > 0 {
                let amount = change.balance_delta as u64;
                account.balance = account
                    .balance
                    .checked_add(amount)
                    .unwrap_or(account.balance);
                supply_delta += change.balance_delta;
            } else if change.balance_delta < 0 {
                let debit = (-change.balance_delta) as u64;
                if account.balance >= debit {
                    account.balance -= debit;
                    supply_delta += change.balance_delta;
                }
            }
            account.sequence_number += change.sequence_increment;
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
                    .unwrap_or(self.total_supply);
            } else {
                let burn_amount = (-supply_delta) as u64;
                if self.total_supply >= burn_amount {
                    self.total_supply -= burn_amount;
                }
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

        // O(1) Global supply tracking
        // token_balance_sets contains FINAL BALANCE snapshots from Move VM, not increments
        for (owner, token_type, amount) in &changeset.token_balance_sets {
            let mut account = self.load_account_or_default(*owner)?;
            let normalized_token_type = Self::normalize_token_type(token_type);

            let old_balance = account.get_token_balance(&normalized_token_type);
            let final_balance = amount.value();

            // Calculate the delta (can be positive or negative)
            let delta: i64 = final_balance as i64 - old_balance as i64;

            // Set balance to final value (this is the snapshot from Move VM)
            account.set_token_balance(normalized_token_type.clone(), amount.clone());
            self.save_account(&account)?;

            // Update global supply with the delta
            let current_supply = self
                .global_token_supplies
                .get(&normalized_token_type)
                .copied()
                .unwrap_or(0);

            let updated_supply = if delta >= 0 {
                current_supply.saturating_add(delta as u64)
            } else {
                current_supply.saturating_sub((-delta) as u64)
            };

            self.global_token_supplies
                .insert(normalized_token_type, updated_supply);
        }

        // บันทึกยอด Global Token Supplies ลงฐานข้อมูลเพียงครั้งเดียวหลังประมวลผลทั้งหมด
        if !changeset.token_balance_sets.is_empty() {
            let supplies_clone = self.global_token_supplies.clone();
            self.save_internal(b"global_token_supplies", &supplies_clone)?;
        }

        for obj_id in &changeset.deleted_objects {
            let obj_key = Self::object_key(obj_id);
            if let Some(existing) = self.load_internal::<CreatedObject>(&obj_key)? {
                let owner_key = Self::owned_objects_key(&existing.owner);
                let mut owned: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
                owned.retain(|x| x != obj_id);
                self.save_internal(&owner_key, &owned)?;
            }
            self.overlay.insert(obj_key, None);
        }

        // 1. ตรวจสอบการสร้าง Object ใหม่เพื่อดัชนี Collection
        for (obj_id, created) in &changeset.created_objects {
            // ถ้าเป็น Object ประเภท Collection ให้จดบันทึกลงดัชนีรวม
            if created.type_.contains("::collection::Collection") {
                let mut index: Vec<String> = self
                    .load_internal(b"nft_collection_index")?
                    .unwrap_or_default();
                if !index.contains(obj_id) {
                    index.push(obj_id.clone());
                    self.save_internal(b"nft_collection_index", &index)?;
                }
            }
        }

        // 2. ตรวจสอบ Events เพื่อดัชนีความสัมพันธ์ NFT <-> Collection
        for event in &changeset.events {
            // ตรวจสอบว่าเป็น MintLog ของ james::nft หรือไม่
            if event.type_tag.to_string().contains("::nft::MintLog") {
                // ข้อมูล MintLog ใน nft.move มี: object_id(32), creator(32), collection_id(32)
                if event.event_data.len() >= 96 {
                    let nft_id_bytes = &event.event_data[0..32];
                    let coll_id_bytes = &event.event_data[64..96];

                    let nft_id = format!("0x{}", hex::encode(nft_id_bytes));
                    let coll_id = format!("0x{}", hex::encode(coll_id_bytes));

                    // บันทึกลงดัชนีสมาชิกของ Collection (O(1) Access)
                    let mut key = b"collection_members:".to_vec();
                    key.extend_from_slice(coll_id.as_bytes());

                    let mut members: Vec<String> = self.load_internal(&key)?.unwrap_or_default();
                    if !members.contains(&nft_id) {
                        members.push(nft_id);
                        self.save_internal(&key, &members)?;
                    }
                }
            }
        }

        for (obj_id, created) in &changeset.created_objects {
            let obj_key = Self::object_key(obj_id);
            let mut new_obj = created.clone();

            if let Ok(Some(existing)) = self.load_internal::<CreatedObject>(&obj_key) {
                new_obj.version = existing.version + 1;
                if new_obj.owner.to_hex_literal() == *obj_id {
                    new_obj.owner = existing.owner;
                }
                if existing.owner != new_obj.owner {
                    let old_owner_key = Self::owned_objects_key(&existing.owner);
                    let mut old_owned: Vec<String> =
                        self.load_internal(&old_owner_key)?.unwrap_or_default();
                    old_owned.retain(|x| x != obj_id);
                    self.save_internal(&old_owner_key, &old_owned)?;
                }
            } else {
                new_obj.version = 1;
            }

            self.save_internal(&obj_key, &new_obj)?;

            let owner_key = Self::owned_objects_key(&new_obj.owner);
            let mut owned: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
            owned.retain(|x| x != obj_id);
            owned.push(obj_id.clone());
            self.save_internal(&owner_key, &owned)?;

            if new_obj.type_.contains("::coin::CoinMetadata<")
                && let Some(start) = new_obj.type_.find('<')
                && let Some(end) = new_obj.type_.rfind('>')
            {
                let token_type = &new_obj.type_[start + 1..end];

                #[derive(Deserialize)]
                struct MoveString {
                    bytes: Vec<u8>,
                }
                #[derive(Deserialize)]
                struct MoveUrl {
                    inner: MoveString,
                }
                #[derive(Deserialize)]
                struct ParsedCoinMetadata {
                    _id: AccountAddress,
                    decimals: u8,
                    name: MoveString,
                    symbol: MoveString,
                    description: MoveString,
                    icon_url: Option<MoveUrl>,
                }

                if let Ok(meta) = bcs::from_bytes::<ParsedCoinMetadata>(&new_obj.data) {
                    let mut key_dec = b"metadata_decimals:".to_vec();
                    key_dec.extend_from_slice(token_type.as_bytes());
                    let _ = self.save_internal(&key_dec, &meta.decimals);

                    if let Ok(name) = String::from_utf8(meta.name.bytes) {
                        let mut key_name = b"metadata_name:".to_vec();
                        key_name.extend_from_slice(token_type.as_bytes());
                        let _ = self.save_internal(&key_name, &name);
                    }

                    if let Ok(symbol) = String::from_utf8(meta.symbol.bytes) {
                        let mut key_sym = b"metadata_symbol:".to_vec();
                        key_sym.extend_from_slice(token_type.as_bytes());
                        let _ = self.save_internal(&key_sym, &symbol);
                    }

                    if let Ok(description) = String::from_utf8(meta.description.bytes) {
                        let mut key_desc = b"metadata_description:".to_vec();
                        key_desc.extend_from_slice(token_type.as_bytes());
                        let _ = self.save_internal(&key_desc, &description);
                    }

                    if let Some(url_obj) = meta.icon_url
                        && let Ok(url) = String::from_utf8(url_obj.inner.bytes)
                    {
                        let mut key_url = b"metadata_icon_url:".to_vec();
                        key_url.extend_from_slice(token_type.as_bytes());
                        let _ = self.save_internal(&key_url, &url);
                    }
                } else if new_obj.data.len() > 32 {
                    let decimals = new_obj.data[32];
                    let mut key = b"metadata_decimals:".to_vec();
                    key.extend_from_slice(token_type.as_bytes());
                    self.save_internal(&key, &decimals)?;
                }
            }
        }

        Ok(())
    }

    /// Get all object IDs owned by an address
    pub fn get_owned_objects(&self, owner: &AccountAddress) -> Result<Vec<String>> {
        let owner_key = Self::owned_objects_key(owner);
        let raw_ids: Vec<String> = self.load_internal(&owner_key)?.unwrap_or_default();
        let mut unique = HashSet::new();
        let mut clean = Vec::new();
        for id in raw_ids {
            if unique.insert(id.clone()) {
                clean.push(id);
            }
        }
        Ok(clean)
    }

    /// Get a specific object by ID
    pub fn get_object(&self, object_id: &str) -> Result<Option<CreatedObject>> {
        let obj_key = Self::object_key(object_id);
        self.load_internal(&obj_key)
    }

    /// Compute the state root hash using SMT
    pub fn compute_state_root(&self) -> Vec<u8> {
        // If SMT is available, use it to compute state root
        if let Some(smt) = &self.smt {
            match smt.root_hash() {
                Ok(root) => return root.to_vec(),
                Err(e) => log::error!("Failed to compute SMT root: {}", e),
            }
        }

        // Fallback: Use default empty state root
        // In production, you should populate SMT with account states
        smt::default_hashes()[0].to_vec()
    }

    /// Validate sequence number for an account
    pub fn validate_sequence(&self, addr: &AccountAddress, expected_seq: u64) -> Result<()> {
        let account_key = Self::account_key(addr);
        if let Some(account) = self.load_internal::<Account>(&account_key)? {
            if account.sequence_number != expected_seq {
                anyhow::bail!("Invalid sequence number");
            }
            Ok(())
        } else {
            if expected_seq != 0 {
                anyhow::bail!("Account does not exist, sequence number must be 0");
            }
            Ok(())
        }
    }

    /// Get the total number of accounts
    pub fn account_count(&self) -> usize {
        // Count all account keys in the overlay and DB
        let mut count = 0;
        let prefix = b"account:";

        // Count from overlay
        for key in self.overlay.keys() {
            if key.starts_with(prefix) {
                count += 1;
            }
        }
        count
    }

    /// Get token decimals for a specific token type
    pub fn get_token_decimals(&self, token_type: &str) -> Result<Option<u8>> {
        let mut key = b"metadata_decimals:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal::<u8>(&key)
    }

    ///  Get token name for a specific token type
    pub fn get_token_name(&self, token_type: &str) -> Result<Option<String>> {
        let mut key = b"metadata_name:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal::<String>(&key)
    }

    ///  Get token symbol for a specific token type
    pub fn get_token_symbol(&self, token_type: &str) -> Result<Option<String>> {
        let mut key = b"metadata_symbol:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal::<String>(&key)
    }

    /// Get token description for a specific token type
    pub fn get_token_description(&self, token_type: &str) -> Result<Option<String>> {
        let mut key = b"metadata_description:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal::<String>(&key)
    }

    /// Get token icon URL for a specific token type
    pub fn get_token_icon_url(&self, token_type: &str) -> Result<Option<String>> {
        let mut key = b"metadata_icon_url:".to_vec();
        key.extend_from_slice(token_type.as_bytes());
        self.load_internal::<String>(&key)
    }
}
