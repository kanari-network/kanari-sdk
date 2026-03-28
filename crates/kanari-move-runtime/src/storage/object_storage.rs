// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::storage::persistent_store::{PersistentStore, PersistentStoreError};
use anyhow::Result;
/// Object Storage Layer for persistent object tracking
/// Stores transferred objects that can be queried and used as function arguments
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Error types that can occur during `ObjectStorage` operations.
#[derive(Debug)]
pub enum ObjectStorageError {
    /// Failed to acquire or interact with an in-memory lock.
    LockError(String),

    /// Failure writing to or reading from the persistent backend.
    PersistenceError(anyhow::Error),

    /// The requested object was not found in the store.
    NotFound,
}

impl ObjectStorageError {
    /// Returns true when this is a lock-related error.
    pub fn is_lock_error(&self) -> bool {
        matches!(self, ObjectStorageError::LockError(_))
    }

    /// Returns true when this is a persistence backend error.
    pub fn is_persistence_error(&self) -> bool {
        matches!(self, ObjectStorageError::PersistenceError(_))
    }

    /// Returns true when the object was not found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ObjectStorageError::NotFound)
    }
}

impl std::fmt::Display for ObjectStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectStorageError::LockError(s) => write!(f, "LockError: {}", s),
            ObjectStorageError::PersistenceError(e) => write!(f, "PersistenceError: {}", e),
            ObjectStorageError::NotFound => write!(f, "NotFound"),
        }
    }
}

impl std::error::Error for ObjectStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ObjectStorageError::PersistenceError(e) => e.source(),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ObjectStorageError {
    fn from(e: anyhow::Error) -> Self {
        ObjectStorageError::PersistenceError(e)
    }
}

impl From<PersistentStoreError> for ObjectStorageError {
    fn from(e: PersistentStoreError) -> Self {
        ObjectStorageError::PersistenceError(e.into())
    }
}

/// Trait abstraction for object storage backends. Allows swapping in-memory and
/// persistent implementations without changing the runtime.
pub trait ObjectStore: Send + Sync {
    fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError>;
    fn get_object(&self, id: &str) -> Option<StoredObject>;
    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject>;
    fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError>;
    fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError>;
    fn count(&self) -> usize;
    fn clear(&self) -> Result<(), ObjectStorageError>;

    // 🚨 เพิ่มบรรทัดนี้ลงไป เพื่อบอกให้ Interface รู้จักฟังก์ชันนี้
    fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &TypeTag,
    ) -> Vec<StoredObject>;
}

/// Stored object with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    pub id: String,
    pub owner: AccountAddress,
    pub type_name: String,
    pub data: Vec<u8>,
    pub version: u64,
}

struct InnerState {
    objects: BTreeMap<String, StoredObject>,
    owner_index: BTreeMap<AccountAddress, Vec<String>>,
}

/// Object Storage - in-memory cache backed by persistent DB
pub struct ObjectStorage {
    // Single lock for atomic updates
    state: Arc<RwLock<InnerState>>,
    // Optional persistent backend
    persistent: Option<Arc<PersistentStore>>,
}

impl ObjectStorage {
    const OBJECT_INDEX_KEY: &'static str = "object_index";

    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(InnerState {
                objects: BTreeMap::new(),
                owner_index: BTreeMap::new(),
            })),
            persistent: None,
        }
    }

    pub fn boxed_inmemory() -> Box<dyn ObjectStore> {
        Box::new(Self::new())
    }
}

impl Default for ObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStorage {
    /// ดึงรายการ Coin ทั้งหมดที่เป็นประเภทเดียวกัน (TypeTag) และเป็นของ Owner คนเดียวกัน
    pub fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &move_core_types::language_storage::TypeTag,
    ) -> Vec<StoredObject> {
        self.get_objects_by_owner(&owner)
            .into_iter()
            .filter(|obj| {
                // Parse Type ของ Object เพื่อเช็คว่าเป็นเหรียญหรือไม่
                if let Ok(struct_tag) = obj
                    .type_name
                    .parse::<move_core_types::language_storage::StructTag>()
                {
                    // เช็คว่าเป็น kanari_system::coin::Coin
                    if struct_tag.module.as_str() == "coin"
                        && struct_tag.name.as_str() == "Coin"
                        && let Some(tag) = struct_tag.type_params.first()
                    {
                        return tag == coin_type;
                    }
                }
                false
            })
            .collect()
    }

    /// Create a new ObjectStorage backed by RocksDB persistence (uses `PersistentStore::open_default`).
    pub fn new_with_persistence() -> Result<Self> {
        let store = PersistentStore::open_default()?;
        let store = Arc::new(store);

        // Load object index (list of ids) if present and rebuild in-memory maps
        let mut objects_map: BTreeMap<String, StoredObject> = BTreeMap::new();

        if let Ok(Some(ids)) = store.load::<Vec<String>>(Self::OBJECT_INDEX_KEY.as_bytes()) {
            for id in ids.into_iter() {
                if let Ok(Some(obj)) =
                    store.load::<StoredObject>(format!("object:{}", id).as_bytes())
                {
                    objects_map.insert(id.clone(), obj);
                }
            }
        }

        // Build owner index from objects
        let mut owner_map: BTreeMap<AccountAddress, Vec<String>> = BTreeMap::new();
        for (id, obj) in objects_map.iter() {
            owner_map.entry(obj.owner).or_default().push(id.clone());
        }

        Ok(Self {
            state: Arc::new(RwLock::new(InnerState {
                objects: objects_map,
                owner_index: owner_map,
            })),
            persistent: Some(store),
        })
    }

    /// Return a boxed persistent `ObjectStore` trait object.
    pub fn boxed_with_persistence() -> Result<Box<dyn ObjectStore>> {
        // Under Miri we avoid filesystem/FFI calls; return in-memory store.
        if cfg!(miri) {
            return Ok(Self::boxed_inmemory());
        }

        Ok(Box::new(Self::new_with_persistence()?))
    }

    /// Store an object
    pub fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError> {
        let id = obj.id.clone();
        let owner = obj.owner;

        // Atomic update of in-memory state
        {
            let mut state = self.state.write().map_err(|e| {
                ObjectStorageError::LockError(format!("Failed to acquire write lock: {}", e))
            })?;

            // Check if object already exists to manage owner index correctly
            let old_owner = state.objects.get(&id).map(|o| o.owner);

            state.objects.insert(id.clone(), obj.clone());

            match old_owner {
                Some(old) => {
                    if old != owner {
                        // Owner changed: remove from old owner's list first
                        if let Some(ids) = state.owner_index.get_mut(&old) {
                            ids.retain(|oid| oid != &id);
                            log::debug!(
                                "Removed object {} from old owner {:?} during transfer",
                                id,
                                old
                            );
                        }
                        // Add to new owner's list
                        state.owner_index.entry(owner).or_default().push(id.clone());
                        log::debug!("Transferred object {} from {:?} to {:?}", id, old, owner);
                    }
                    // If owner is same, do nothing (id is already in the list)
                }
                None => {
                    // New object: add to owner index
                    state.owner_index.entry(owner).or_default().push(id.clone());
                    log::debug!("Created new object {} for owner {:?}", id, owner);
                }
            }
        }

        // Persist to DB if available
        if let Some(store) = &self.persistent {
            // save object
            store.save(format!("object:{}", id).as_bytes(), &obj)?;

            // update object index
            let mut ids = store
                .load::<Vec<String>>(Self::OBJECT_INDEX_KEY.as_bytes())?
                .unwrap_or_default();

            if !ids.iter().any(|x| x == &id) {
                ids.push(id.clone());
                store.save(Self::OBJECT_INDEX_KEY.as_bytes(), &ids)?;
            }
        }

        Ok(())
    }

    /// Get object by ID
    pub fn get_object(&self, id: &str) -> Option<StoredObject> {
        // Prefer in-memory
        if let Some(obj) = self
            .state
            .read()
            .ok()
            .and_then(|state| state.objects.get(id).cloned())
        {
            return Some(obj);
        }

        // If not present and persistent enabled, try loading from DB
        if let Some(obj) = self.persistent.as_ref().and_then(|store| {
            store
                .load::<StoredObject>(format!("object:{}", id).as_bytes())
                .ok()
                .flatten()
        }) {
            // populate in-memory caches
            if let Ok(mut state) = self.state.write() {
                // Double check if it was added while we were loading
                if let Some(existing) = state.objects.get(id) {
                    return Some(existing.clone());
                }

                state.objects.insert(id.to_string(), obj.clone());
                state
                    .owner_index
                    .entry(obj.owner)
                    .or_default()
                    .push(id.to_string());
            }
            return Some(obj);
        }
        None
    }

    /// Get all objects owned by an address
    pub fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let object_ids = match state.owner_index.get(owner) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        object_ids
            .iter()
            .filter_map(|id| state.objects.get(id).cloned())
            .collect()
    }

    /// Update object ownership
    pub fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError> {
        // Ensure object is loaded into memory (if it exists on disk)
        if self.get_object(id).is_none() {
            return Err(ObjectStorageError::NotFound);
        }

        // Atomic update in memory and prepare for persistence
        let obj_to_persist = {
            let mut state = self.state.write().map_err(|e| {
                ObjectStorageError::LockError(format!("Failed to acquire write lock: {}", e))
            })?;

            let old_owner = {
                let obj = state
                    .objects
                    .get_mut(id)
                    .ok_or(ObjectStorageError::NotFound)?;
                let old = obj.owner;
                obj.owner = new_owner;
                old
            };

            // Update owner indices
            // Remove from old owner
            if let Some(ids) = state.owner_index.get_mut(&old_owner) {
                ids.retain(|oid| oid != id);
            }
            // Add to new owner
            state
                .owner_index
                .entry(new_owner)
                .or_default()
                .push(id.to_string());

            // Clone the object while we hold the lock to persist it safely
            state
                .objects
                .get(id)
                .cloned()
                .ok_or(ObjectStorageError::NotFound)?
        };

        // Persist updated object if available
        if let Some(store) = &self.persistent {
            store.save(format!("object:{}", id).as_bytes(), &obj_to_persist)?;
        }

        Ok(())
    }

    /// Delete object
    pub fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError> {
        // Atomic update in memory
        let _ = {
            let mut state = self.state.write().map_err(|e| {
                ObjectStorageError::LockError(format!("Failed to acquire write lock: {}", e))
            })?;

            if let Some(obj) = state.objects.remove(id) {
                // Remove from owner index
                if let Some(ids) = state.owner_index.get_mut(&obj.owner) {
                    ids.retain(|oid| oid != id);
                }
                Some(obj.owner)
            } else {
                None
            }
        };

        // Persist deletion if available
        if let Some(store) = &self.persistent {
            // delete object
            store.delete(format!("object:{}", id).as_bytes())?;

            // update object index
            let mut ids = store
                .load::<Vec<String>>(Self::OBJECT_INDEX_KEY.as_bytes())?
                .unwrap_or_default();

            if let Some(pos) = ids.iter().position(|x| x == id) {
                ids.remove(pos);
                store.save(Self::OBJECT_INDEX_KEY.as_bytes(), &ids)?;
            }
        }

        Ok(())
    }

    /// Get total number of objects
    pub fn count(&self) -> usize {
        self.state.read().map(|s| s.objects.len()).unwrap_or(0)
    }

    /// Clear all objects
    pub fn clear(&self) -> Result<(), ObjectStorageError> {
        let mut state = self.state.write().map_err(|e| {
            ObjectStorageError::LockError(format!("Failed to acquire write lock: {}", e))
        })?;
        state.objects.clear();
        state.owner_index.clear();

        // Note: This does not clear persistence!
        // If we wanted to clear persistence, we'd need to iterate keys.
        // For now, assuming clear() is used for in-memory reset or testing.

        Ok(())
    }
}

// Implement the ObjectStore trait for the in-memory ObjectStorage so it can
// be used as a boxed trait object by the runtime.
impl ObjectStore for ObjectStorage {
    fn store_object(&self, obj: StoredObject) -> Result<(), ObjectStorageError> {
        ObjectStorage::store_object(self, obj)
    }

    fn get_object(&self, id: &str) -> Option<StoredObject> {
        ObjectStorage::get_object(self, id)
    }

    fn get_objects_by_owner(&self, owner: &AccountAddress) -> Vec<StoredObject> {
        ObjectStorage::get_objects_by_owner(self, owner)
    }

    fn transfer_object(
        &self,
        id: &str,
        new_owner: AccountAddress,
    ) -> Result<(), ObjectStorageError> {
        ObjectStorage::transfer_object(self, id, new_owner)
    }

    fn delete_object(&self, id: &str) -> Result<(), ObjectStorageError> {
        ObjectStorage::delete_object(self, id)
    }

    fn count(&self) -> usize {
        ObjectStorage::count(self)
    }

    fn clear(&self) -> Result<(), ObjectStorageError> {
        ObjectStorage::clear(self)
    }

    // 🚨 เพิ่มบล็อกนี้ต่อท้ายสุด (ก่อนปิดปีกกาของ impl)
    fn get_coins_by_type_and_owner(
        &self,
        owner: AccountAddress,
        coin_type: &TypeTag,
    ) -> Vec<StoredObject> {
        ObjectStorage::get_coins_by_type_and_owner(self, owner, coin_type)
    }
}
